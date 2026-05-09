use std::{char, cmp::min};

#[cfg(feature = "global_static")]
use crate::PhoneNumberUtil;
use crate::{
    InternalError, PhoneNumber, PhoneNumberFormat, Region,
    generated::uniprops_digits,
    interfaces::{AsOriginal, LenWrite, OptionalHasher},
    panic_internal,
    phonenumber_mask::{
        MaskDigitsConfig, MaxHashedLengthExceededError,
        helper_types::{self, LenWriteString},
    },
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
    PhoneMaskUtil {
        ret: Self -> Self => | v | Self { inner: v },
        ret: Result<String, InternalError<std::convert::Infallible>> -> String => | v | {
            v.map_err(panic_internal).unwrap_or_else(| err | match err {})
        }
    },

    PhoneMaskUtilFallible {
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

    /// Masks digits in a raw phone number string and writes the output directly to the provided writer.
    ///
    /// This method is memory-efficient and uses a zero-allocation strategy by predicting
    /// the required capacity and utilizing the `LenWrite` sink.
    ///
    /// Extracted extensions (e.g., `;ext=123`) and RFC3966 `phone-context` URIs are fully masked automatically.
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

    /// Generates a semantic XML-like token representing the parsed phone number.
    ///
    /// # Hasher Parameter
    /// * Pass an implementation of `PhoneHasher` to generate and append a cryptographic hash.
    /// * Pass `()` to omit hashing. The token will only include the country mapping.
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

        // Hash phone if a valid hasher is provided, otherwise evaluate to None
        let hashed = hasher.hash_phone(phone)?;

        let len = if let Some(ref h) = hashed {
            SEMANTIC_TOKEN_DEFAULT_LEN + SEMANTIC_TOKEN_HASH.len() + h.len() * 2
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
            writer.write_all(hashed.as_hex(&mut buf).as_bytes())?;
        }

        writer.write_all(SEMANTIC_TOKEN_END.as_bytes())?;

        Ok(())
    }

    /// Formats the given phone number strictly according to the provided `PhoneNumberFormat`,
    /// then partially obscures the output based on `MaskDigitsConfig`.
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

    /// A convenience method that masks the digits of a raw phone string and allocates a new `String`.
    ///
    /// For memory-sensitive contexts, consider using the `mask_digits` method with a custom sink.
    #[export]
    pub fn mask_digits_to_string(&self, raw_input: &str, config: MaskDigitsConfig) -> String {
        let mut writer = LenWriteString::new();

        self.mask_digits(raw_input, config, &mut writer)
            .expect("In-memory string write should never fail");

        writer.into()
    }

    /// A convenience method that generates a semantic token string and allocates a new `String`.
    ///
    /// You can supply either an active `PhoneHasher` or `()` to skip the hashing process entirely.
    #[export]
    pub fn tokenize_to_string(
        &self,
        phone: &PhoneNumber,
        hasher: impl OptionalHasher,
    ) -> helper_types::Result<String> {
        let mut writer = LenWriteString::new();

        if let Err(err) = self.tokenize(phone, hasher, &mut writer) {
            return Err(err
                .downcast::<MaxHashedLengthExceededError>()
                .expect("In-memory string write should never fail"));
        }

        Ok(writer.into())
    }

    /// Counts the number of recognizable digits and alphabetic characters (e.g. vanity numbers) in a string.
    fn count_digits(s: &str) -> usize {
        s.chars()
            .filter(|c| {
                uniprops_digits::uniprops::get_digit_value(*c).is_some() || c.is_ascii_alphabetic()
            })
            .count()
    }
}

#[cfg(feature = "global_static")]
impl PhoneMaskUtil<PhoneNumberUtil, &'static PhoneNumberUtil> {
    pub fn new() -> Self {
        use crate::PHONE_NUMBER_UTIL;

        Self {
            inner: PhoneMaskUtilInternal::new_for_util(&PHONE_NUMBER_UTIL),
        }
    }
}

#[cfg(feature = "global_static")]
impl Default for PhoneMaskUtil<PhoneNumberUtil, &'static PhoneNumberUtil> {
    fn default() -> Self {
        Self::new()
    }
}
