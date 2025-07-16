use std::borrow::Cow;

use crate::generated::proto::phonenumber::PhoneNumber;
use super::{
    phonenumberutil_internal::PhoneNumberUtilInternal,
    enums::PhoneNumberFormat
};

pub struct PhoneNumberUtil {
    util_internal: PhoneNumberUtilInternal
}

impl PhoneNumberUtil {
    pub fn format<'a>(&self, phone_number: &'a PhoneNumber, number_format: PhoneNumberFormat) -> Cow<'a, str> {
        self.util_internal
            .format(phone_number, number_format)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    pub fn format_in_original_format<'a>(
        &self, phone_number: &'a PhoneNumber, region_calling_from: impl AsRef<str>
    ) -> Cow<'a, str> {
        self.util_internal
            .format_in_original_format(phone_number, region_calling_from.as_ref())
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }


}