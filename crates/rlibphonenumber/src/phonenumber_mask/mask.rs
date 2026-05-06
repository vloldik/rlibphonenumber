use std::{borrow::Cow, char, cmp::min, hash::Hasher};

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

/// Configuration for character-by-character digit masking.
#[derive(Debug, Clone, Copy)]
pub struct MaskDigitsConfig {
    /// The UTF-8 character used to obscure sensitive digits (e.g., `'*'`, `'X'`, or `'🔒'`).
    mask_char: char,
    /// If `true`, the last up to 4 digits of the main number are preserved (left unmasked).
    /// If `false`, all digits are masked.
    keep_last: bool,
}

/// A stack-allocated buffer capable of storing up to 64 bytes of hash data.
///
/// Designed to avoid heap allocations when working with hash outputs (e.g., SHA-256, HMAC).
#[derive(Clone, Copy)]
pub struct Hashed([u8; 64], usize);

/// Defines the strategy used to obscure or pseudonymize a phone number.
#[derive(Debug, Clone)]
pub enum MaskType {
    /// Completely replaces the phone number with a static, pre-defined string (e.g., `"REDACTED"`).
    Fixed(Cow<'static, str>),
    /// Applies fine-grained masking to digits while preserving formatting and symbols.
    MaskDigits(MaskDigitsConfig),
    /// Replaces the number with a semantic XML-like token (e.g., `<Phone country="US">`).
    /// Optionally embeds a hashed representation for secure analytics tracking.
    SemanticToken {
        /// The regional origin of the phone number.
        region: Option<Region>,
        /// An optional hash to uniquely identify the tokenized entity without exposing PII.
        add_hash: Option<Hashed>,
    },
    /// Replaces the phone number entirely with its hex-encoded hash.
    Hash(Hashed),
}

/// Helper function to convert a `Hashed` instance into a lowercase hexadecimal string.
///
/// # Arguments
/// * `hashed` - The hashed byte array.
/// * `buf` - A mutable reference to a 128-byte array used as a backing buffer.
///
/// # Returns
/// A string slice referencing the hex-encoded portion of the buffer.
pub fn hash_to_hex<'a>(hashed: &Hashed, buf: &'a mut [u8; 128]) -> &'a str {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let bytes = hashed.as_slice();

    for (i, &byte) in bytes.iter().enumerate() {
        buf[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        buf[i * 2 + 1] = HEX_CHARS[(byte & 0xf) as usize];
    }

    let hex_len = bytes.len() * 2;
    // SAFETY: The buffer is filled strictly with ASCII characters ('0'-'9' and 'a'-'f').
    // It is mathematically guaranteed to be valid UTF-8. No original undefined bytes
    // from the buffer are exposed, because the slice is bounded by `hex_len`.
    unsafe { std::str::from_utf8_unchecked(&buf[..hex_len]) }
}

/// Counts the number of recognizable digits and alphabetic characters (vanity numbers) in a string.
fn count_digits(s: &str) -> usize {
    s.chars()
        .filter(|c| {
            uniprops_digits::uniprops::get_digit_value(*c).is_some() || c.is_ascii_alphabetic()
        })
        .count()
}

/// Masks or pseudonymizes a raw phone number string according to the provided strategy.
///
/// This function carefully traverses the string to differentiate between the core phone
/// number, non-significant formatting, extensions, and `RFC3966` URI context.
/// Memory allocation is optimized by explicitly calling `writer.grow()` once before writing.
///
/// # Arguments
/// * `util` - The underlying phone number utility implementation.
/// * `raw_input` - The raw phone number string (e.g., `"+1 (918) 123-4567"` or `tel:...`).
/// * `mask` - The rule set (`MaskType`) applied to the data.
/// * `writer` - A destination writer implementing `LenWrite` to receive the masked output.
///
/// # Errors
/// Returns an `std::io::Result` if writing to the output stream fails.
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

            // Locate any user-provided extensions (e.g. "ext 123" or "доб. 123")
            let ext = util
                .as_original()
                .reg_exps
                .extn_pattern
                .captures(raw_input)
                .and_then(|c| c.iter().skip(1).flatten().find(|m| !m.is_empty()));

            let ext_pos: Option<usize> = ext.map(|ext| ext.start());

            // Handle RFC3966 URI `phone-context` parameters
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

            // Determine where the main number ends, respecting extensions and contexts.
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

            // Pre-allocate the exact required capacity (zero-allocation strategy)
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

                // Determine whether the current digit should be obscured
                let should_mask = if in_ext || in_ctx {
                    true // Always fully mask extensions and context URIs
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
    /// Creates a new configuration for digit masking.
    pub fn new(mask_char: char, keep_last: bool) -> Self {
        Self {
            mask_char,
            keep_last,
        }
    }
}

impl Default for MaskDigitsConfig {
    /// Default configuration: masks using `'*'` and preserves the last 4 digits.
    fn default() -> Self {
        Self::new('*', true)
    }
}

impl Hashed {
    /// Creates a `Hashed` instance using a standard Rust `Hasher` (e.g., `DefaultHasher`, `SipHash`).
    pub fn from_hasher(hasher: impl Hasher, phone: &PhoneNumber) -> Self {
        PhoneStdHasher(hasher).hash_phone(phone).unwrap() // Always returns Some() since hasher returns u64
    }

    /// Creates a `Hashed` instance using a cryptographic digest (e.g., `Sha256`).
    ///
    /// Returns `None` if the chosen digest's output exceeds 64 bytes.
    #[cfg(feature = "digest")]
    pub fn from_digest(digest: impl Digest + Update, phone: &PhoneNumber) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        PhoneDigestHasher(digest).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a cryptographic digest seeded with salt bytes.
    #[cfg(feature = "digest")]
    pub fn from_salted_digest(
        digest: impl Digest + Update,
        salt: &[u8],
        phone: &PhoneNumber,
    ) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        PhoneDigestHasher::new_with_salt(digest, salt).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a Message Authentication Code (e.g., `HMAC`).
    #[cfg(feature = "digest_mac")]
    pub fn from_mac(mac: impl Mac + Update, phone: &PhoneNumber) -> Option<Self> {
        use crate::phonenumber_mask::hash::PhoneMacHasher;
        PhoneMacHasher(mac).hash_phone(phone)
    }

    /// Constructs a `Hashed` buffer from a raw byte slice.
    ///
    /// # Returns
    /// Returns `Some(Self)` if `bytes.len() <= 64`. Returns `None` otherwise.
    pub fn from_slice(bytes: impl AsRef<[u8]>) -> Option<Self> {
        let len = bytes.as_ref().len();
        if len > 64 {
            return None;
        }

        let mut buf = [0u8; 64];
        buf[..len].copy_from_slice(bytes.as_ref());

        Some(Self(buf, len))
    }

    /// Retrieves the underlying payload as a slice of bytes.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.1]
    }
}

// Standard trait implementations for `Hashed`
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
