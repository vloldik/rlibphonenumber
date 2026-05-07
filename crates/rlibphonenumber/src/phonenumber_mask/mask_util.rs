use std::{char, cmp::min};

use crate::{
    InternalError, PhoneNumber, PhoneNumberFormat, Region,
    generated::uniprops_digits,
    interfaces::{AsOriginal, LenWrite, OptionalHasher, PhoneHasher},
    panic_internal,
    phonenumber_mask::{Hashed, MaskDigitsConfig, helper_types},
    phonenumberutil::{
        helper_constants::{PLUS_CHARS, RFC3966_PHONE_CONTEXT},
        phonenumberutil_internal::PhoneNumberUtilInternal,
    },
};
use std::ops::Deref;

use rlibphonenumber_macro::{export, public_wrapper};

#[derive(Debug, Clone)]
pub struct PhoneMaskUtilInternal<U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>> {
    util: T,
}

#[public_wrapper(
    MaskUtil {
        ret: Self -> Self => | v | Self { inner: v },
        ret: Result<String, InternalError<std::convert::Infallible>> -> String => | v | {
            v.map_err(panic_internal).unwrap_or_else(| err | match err {})
        }
    },

    MaskUtilFallible {
        ret: Self -> Self => | v | Self { inner: v },
    }
)]

impl<U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>> PhoneMaskUtilInternal<U, T> {
    #[export]
    pub fn new_for_util(util: T) -> Self {
        Self { util }
    }

    #[inline]
    fn phone_util(&self) -> &PhoneNumberUtilInternal {
        self.util.as_original()
    }

    #[export]
    fn mask_digits(
        &self,
        raw_input: &str,
        config: MaskDigitsConfig,
        writer: &mut dyn LenWrite,
    ) -> std::io::Result<()> {
        let start = raw_input
            .find(|c: char| {
                uniprops_digits::uniprops::get_digit_value(c).is_some() || PLUS_CHARS.contains(c)
            })
            .unwrap_or(0);

        // Locate any user-provided extensions (e.g. "ext 123" or "доб. 123")
        let ext = self
            .phone_util()
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
        let total_main_digits = Self::count_digits(main_part);
        let suffix_len = min(
            total_main_digits.saturating_sub(config.min_masked),
            config.max_unmasked,
        );

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
            let is_digit =
                uniprops_digits::uniprops::get_digit_value(c).is_some() || c.is_ascii_alphabetic();

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

    #[export]
    fn tokenize(
        &self,
        phone: &PhoneNumber,
        hasher: impl OptionalHasher,
        writer: &mut dyn LenWrite,
    ) -> std::io::Result<()> {
        const SEMANTIC_TOKEN_START: &str = "<Phone country=\"";
        const SEMANTIC_TOKEN_HASH: &str = "\" hash=\"";
        const SEMANTIC_TOKEN_END: &str = "\">";
        const SEMANTIC_TOKEN_DEFAULT_LEN: usize = 16 + 3 + 2;

        let hashed = hasher.hash_phone(phone)?;

        let len = if let Some(hashed) = hashed {
            SEMANTIC_TOKEN_DEFAULT_LEN + SEMANTIC_TOKEN_HASH.len() + hashed.len() * 2
        } else {
            SEMANTIC_TOKEN_DEFAULT_LEN
        };

        writer.grow(len);

        writer.write_all(SEMANTIC_TOKEN_START.as_bytes())?;
        if let Some(country_code) = self
            .phone_util()
            .get_region_for_country_code(phone.country_code)
        {
            writer.write_all(country_code.as_region_str().as_bytes())?;
        } else {
            writer.write_all(Region::World.as_region_str().as_bytes())?;
        }
        if let Some(hashed) = hashed {
            writer.write_all(SEMANTIC_TOKEN_HASH.as_bytes())?;
            let mut buf = [0; 128];
            writer.write_all(Self::hash_to_hex(&hashed, &mut buf).as_bytes())?;
        }
        writer.write_all(SEMANTIC_TOKEN_END.as_bytes())?;

        Ok(())
    }

    #[export]
    pub fn hash_number_to_string(
        &self,
        phone: &PhoneNumber,
        hasher: impl PhoneHasher,
    ) -> helper_types::Result<String> {
        let hashed = hasher.hash_phone(phone)?;
        let mut buf = [0u8; 128];
        Ok(Self::hash_to_hex(&hashed, &mut buf).to_string())
    }

    #[export]
    pub fn format_and_mask(
        &self,
        phone: &PhoneNumber,
        format: PhoneNumberFormat,
        config: MaskDigitsConfig,
    ) -> Result<String, InternalError<std::convert::Infallible>> {
        let formatted = self.phone_util().format(phone, format)?;

        Ok(self.mask_digits_to_string(&formatted, config))
    }

    #[export]
    pub fn mask_digits_to_string(&self, raw_input: &str, config: MaskDigitsConfig) -> String {
        let mut writer = String::new();

        self.mask_digits(&raw_input, config, &mut writer)
            .expect("In-memory write should never fail");

        writer
    }

    #[export]
    pub fn tokenize_to_string(&self, phone: &PhoneNumber, hasher: impl OptionalHasher) -> String {
        let mut writer = String::new();

        self.tokenize(phone, hasher, &mut writer)
            .expect("In-memory write should never fail");

        writer
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
}
