use std::ops::Deref;

use rlibphonenumber_macro::{export, public_wrapper};

use crate::{
    PhoneNumber,
    enums::PhoneNumberFormat,
    errors::InternalError,
    interfaces::{AsOriginal, PhoneHasher},
    panic_internal,
    phonenumber_mask::{MaskDigitsConfig, MaskType, mask::hash_to_hex, mask_number},
    phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
};

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
    pub fn new(util: T) -> Self {
        Self { util }
    }

    #[inline]
    fn phone_util(&self) -> &PhoneNumberUtilInternal {
        self.util.as_original()
    }

    #[export]
    pub fn hash_number_to_string(
        &self,
        hasher: impl PhoneHasher,
        phone: &PhoneNumber,
    ) -> Option<String> {
        let hashed = hasher.hash_phone(phone)?;
        let mut buf = [0u8; 128];
        Some(hash_to_hex(&hashed, &mut buf).to_owned())
    }

    #[export]
    pub fn mask_phone_to_string(
        &self,
        raw_input: &str,
        mask: &MaskType,
    ) -> std::io::Result<String> {
        let mut buf = String::new();
        mask_number(self.phone_util(), raw_input, mask, &mut buf)?;
        Ok(buf)
    }

    #[export]
    pub fn format_and_mask(
        &self,
        phone: &PhoneNumber,
        format: PhoneNumberFormat,
        mask: &MaskType,
    ) -> Result<String, InternalError<std::convert::Infallible>> {
        let formatted = self.phone_util().format(phone, format)?;
        Ok(self
            .mask_phone_to_string(&formatted, mask)
            .expect("In-memory write should never fail"))
    }

    #[export]
    pub fn mask_fixed_to_string(
        &self,
        raw_input: &str,
        replacement: std::borrow::Cow<'static, str>,
    ) -> std::io::Result<String> {
        self.mask_phone_to_string(raw_input, &MaskType::Fixed(replacement))
    }

    #[export]
    pub fn mask_digits_to_string(
        &self,
        raw_input: &str,
        config: MaskDigitsConfig,
    ) -> std::io::Result<String> {
        self.mask_phone_to_string(raw_input, &MaskType::MaskDigits(config))
    }

    #[export]
    pub fn tokenize(
        &self,
        phone: &PhoneNumber,
        hasher: Option<impl PhoneHasher>,
    ) -> Result<String, InternalError<std::convert::Infallible>> {
        let hashed = hasher.and_then(|h| h.hash_phone(phone));
        let region = self
            .phone_util()
            .get_region_for_country_code(phone.country_code);

        let mask = MaskType::SemanticToken {
            region,
            add_hash: hashed,
        };

        let mut writer = String::new();

        mask_number(self.phone_util(), "", &mask, &mut writer)
            .expect("In-memory write should never fail");

        Ok(writer)
    }
}
