use std::{char, cmp::min, hash::Hasher};

#[cfg(feature = "digest_mac")]
use digest::Mac;
#[cfg(feature = "digest")]
use digest::{Digest, Update};

use crate::{
    PhoneNumber, Region,
    generated::uniprops_digits,
    interfaces::{AsOriginal, LenWrite, PhoneHasher},
    phonenumber_mask::hash::PhoneStdHasher,
    phonenumberutil::{
        helper_constants::{PLUS_CHARS, RFC3966_PHONE_CONTEXT},
        phonenumberutil_internal::PhoneNumberUtilInternal,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct MaskDigitsConfig {
    mask_char: char,
    keep_last: bool,
}

#[derive(Clone, Copy)]
pub struct Hashed([u8; 64], usize);

#[derive(Debug, Clone)]
pub enum MaskType {
    Fixed(String),
    MaskDigits(MaskDigitsConfig),
    SemanticToken {
        region: Option<Region>,
        add_hash: Option<Hashed>,
    },
    Hash(Hashed),
}

fn hash_to_hex<'a>(hashed: &Hashed, buf: &'a mut [u8; 128]) -> &'a str {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let bytes = hashed.as_slice();

    for (i, &byte) in bytes.iter().enumerate() {
        buf[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        buf[i * 2 + 1] = HEX_CHARS[(byte & 0xf) as usize];
    }

    let hex_len = bytes.len() * 2;
    // SAFETY: Buffer is fulfilled with only '0'-'9' and 'a'-'f',
    // and is guaranteed to contain only valid ASCII/UTF-8 chars.
    // There is no chance we can take symbols from original buf, because it
    // is overridden at 0..(bytes.len*2 = hex_len)
    unsafe { std::str::from_utf8_unchecked(&buf[..hex_len]) }
}

fn count_digits(s: &str) -> usize {
    s.chars()
        .filter(|c| {
            uniprops_digits::uniprops::get_digit_value(*c).is_some() || c.is_ascii_alphabetic()
        })
        .count()
}

pub fn mask_number(
    util: &impl AsOriginal<PhoneNumberUtilInternal>,
    raw_input: &str,
    mask: &MaskType,
    writer: &mut dyn LenWrite,
) -> std::io::Result<()> {
    match mask {
        MaskType::Fixed(fixed) => Ok(writer.write_all(fixed.as_bytes())?),
        MaskType::MaskDigits(config) => {
            let start = raw_input
                .find(|c: char| {
                    uniprops_digits::uniprops::get_digit_value(c).is_some()
                        || PLUS_CHARS.contains(c)
                })
                .unwrap_or(0);

            let ext = util
                .as_original()
                .reg_exps
                .extn_pattern
                .captures(raw_input)
                .and_then(|c| c.iter().skip(1).flatten().find(|m| !m.is_empty()));

            let ext_pos: Option<usize> = ext.map(|ext| ext.start());

            let ctx_pos: Option<usize> = raw_input
                .find(RFC3966_PHONE_CONTEXT)
                .map(|i| i + RFC3966_PHONE_CONTEXT.len());
            let ctx_end: usize = ctx_pos
                .map(|p| {
                    raw_input[p..]
                        .find(';')
                        .map(|i| p + i)
                        .unwrap_or(raw_input.len())
                })
                .unwrap_or(0);
            let ctx_char_count = ctx_pos
                .map(|p| raw_input[p..ctx_end].chars().count())
                .unwrap_or(0);

            let main_end = ext_pos.unwrap_or(raw_input.len()).min(
                ctx_pos
                    .map(|p| p - RFC3966_PHONE_CONTEXT.len())
                    .unwrap_or(raw_input.len()),
            );

            let main_part = &raw_input[start.min(main_end)..main_end];
            let total_main_digits = count_digits(main_part);
            let suffix_len = if config.keep_last {
                min(total_main_digits.saturating_sub(3), 4)
            } else {
                0
            };
            let mask_count = total_main_digits.saturating_sub(suffix_len);
            let mut digit_seen = 0usize;
            let mut char_buf = [0u8; 4];
            let mut mask_buf = [0u8; 4];
            let mask_bytes = config.mask_char.encode_utf8(&mut mask_buf).as_bytes();

            writer.grow(
                raw_input.len()
                    + mask_count * config.mask_char.len_utf8()
                    + ctx_char_count * config.mask_char.len_utf8(),
            );

            for (byte_pos, c) in raw_input.char_indices() {
                if byte_pos < start {
                    writer.write_all(c.encode_utf8(&mut char_buf).as_bytes())?;
                    continue;
                }

                let in_ctx = ctx_pos
                    .map(|p| (p..ctx_end).contains(&byte_pos))
                    .unwrap_or(false);
                let is_digit = uniprops_digits::uniprops::get_digit_value(c).is_some()
                    || c.is_ascii_alphabetic();

                if !is_digit && !in_ctx {
                    writer.write_all(c.encode_utf8(&mut char_buf).as_bytes())?;
                    continue;
                }

                let in_main = byte_pos < main_end;
                let in_ext = ext_pos.map(|p| byte_pos >= p).unwrap_or(false);

                let should_mask = if in_ext || in_ctx {
                    true
                } else if in_main {
                    digit_seen += 1;
                    digit_seen <= mask_count
                } else {
                    false
                };

                if should_mask {
                    writer.write_all(mask_bytes)?;
                } else {
                    writer.write_all(c.encode_utf8(&mut char_buf).as_bytes())?;
                }
            }

            Ok(())
        }
        MaskType::SemanticToken { region, add_hash } => {
            const SEMANTIC_TOKEN_START: &str = "<Phone country=\"";
            const SEMANTIC_TOKEN_HASH: &str = "\" hash=\"";
            const SEMANTIC_TOKEN_END: &str = "\">";
            const SEMANTIC_TOKEN_DEFAULT_LEN: usize = 16 + 3 + 2;

            let len = if let Some(hashed) = add_hash {
                SEMANTIC_TOKEN_DEFAULT_LEN + SEMANTIC_TOKEN_HASH.len() + hashed.1 * 2
            } else {
                SEMANTIC_TOKEN_DEFAULT_LEN
            };

            writer.grow(len);

            writer.write_all(SEMANTIC_TOKEN_START.as_bytes())?;
            if let Some(country_code) = region {
                writer.write_all(country_code.as_region_str().as_bytes())?;
            } else {
                writer.write_all(Region::World.as_region_str().as_bytes())?;
            }
            if let Some(hashed) = add_hash {
                writer.write_all(SEMANTIC_TOKEN_HASH.as_bytes())?;
                let mut buf = [0; 128];
                writer.write_all(hash_to_hex(hashed, &mut buf).as_bytes())?;
            }
            writer.write_all(SEMANTIC_TOKEN_END.as_bytes())?;

            Ok(())
        }
        MaskType::Hash(hashed) => {
            let mut buf = [0; 128];
            writer.write_all(hash_to_hex(hashed, &mut buf).as_bytes())?;
            Ok(())
        }
    }
}

impl MaskDigitsConfig {
    pub fn new(mask_char: char, keep_last: bool) -> Self {
        Self {
            mask_char,
            keep_last,
        }
    }
}

impl Default for MaskDigitsConfig {
    fn default() -> Self {
        Self::new('*', true)
    }
}

impl Hashed {
    pub fn from_hasher(hasher: impl Hasher, phone: &PhoneNumber) -> Self {
        PhoneStdHasher(hasher).hash_phone(phone).unwrap() // Always returns Some() since hasher returns u64
    }

    #[cfg(feature = "digest")]
    pub fn from_digest(digest: impl Digest + Update, phone: &PhoneNumber) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;

        PhoneDigestHasher(digest).hash_phone(phone)
    }

    #[cfg(feature = "digest")]
    pub fn from_salted_digest(
        digest: impl Digest + Update,
        salt: &[u8],
        phone: &PhoneNumber,
    ) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;

        PhoneDigestHasher::new_with_salt(digest, salt).hash_phone(phone)
    }

    #[cfg(feature = "digest_mac")]
    pub fn from_mac(mac: impl Mac + Update, phone: &PhoneNumber) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneMacHasher;

        PhoneMacHasher(mac).hash_phone(phone)
    }

    pub fn from_slice(bytes: impl AsRef<[u8]>) -> Option<Self> {
        let len = bytes.as_ref().len();
        if len > 64 {
            return None;
        }

        let mut buf = [0u8; 64];
        buf[..len].copy_from_slice(bytes.as_ref());

        Some(Self(buf, len))
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.1]
    }
}

impl PartialEq for Hashed {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl Eq for Hashed {}

impl std::hash::Hash for Hashed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl std::fmt::Debug for Hashed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hash(\"")?;
        for byte in self.as_slice() {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "\")")
    }
}

impl AsRef<[u8]> for Hashed {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
