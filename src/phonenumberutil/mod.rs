mod helper_constants;
pub mod helper_functions;
mod enums;
mod phonenumberutil;
mod regex_and_mappings;

pub use enums::{MatchType, PhoneNumberFormat, PhoneNumberType, ValidationResultErr, ValidNumberLenType};
use thiserror::Error;

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
