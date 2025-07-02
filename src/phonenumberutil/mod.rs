mod helper_constants;
pub mod helper_functions;
mod errors;
mod enums;
mod phonenumberutil;
mod phone_number_regexps_and_mappings;

use std::sync::LazyLock;

pub use enums::{MatchType, PhoneNumberFormat, PhoneNumberType, ValidationResultErr, ValidNumberLenType};
use thiserror::Error;

use crate::phonenumberutil::phonenumberutil::PhoneNumberUtil;

#[derive(Debug, Error)]
pub enum ErrorType {
    #[error("No parsing")]
    NoParsingError,
    #[error("Invalid country code")]
    InvalidCountryCodeError, // INVALID_COUNTRY_CODE in the java version.
    #[error("Not a number")]
    NotANumber,
    #[error("Too short after idd")]
    TooShortAfterIdd,
    #[error("Too short Nsn")]
    TooShortNsn,
    #[error("Too long nsn")]
    TooLongNsn, // TOO_LONG in the java version.
}

static PHONE_NUMBER_UTIL: LazyLock<PhoneNumberUtil> = LazyLock::new(|| {
    PhoneNumberUtil::new()
});