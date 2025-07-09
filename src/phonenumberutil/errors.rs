use core::error;
use std::num::ParseIntError;

use thiserror::Error;

use crate::regexp_cache::ErrorInvalidRegex;

#[derive(Debug, PartialEq, Error)]
pub enum PhoneNumberUtilError {
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex),
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("Extract number error: {0}")]
    ExtractNumberError(#[from] ExtractNumberError)
}   

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    // Removed as OK variant
    // NoParsingError,
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
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex),
    #[error("{0}")]
    ParseNumberAsIntError(#[from] ParseIntError),
    #[error("{0}")]
    ExtractNumberError(#[from] ExtractNumberError),
}

#[derive(Debug, PartialEq, Error)]
pub enum ExtractNumberError {
    #[error("No valid start character found")]
    NoValidStartCharacter,
    #[error("Invalid number")]
    NotANumber,
}

#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberError {
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex),
    #[error("No example number")]
    NoExampleNumberError,
    #[error("Could not get number")]
    CouldNotGetNumberError,
    #[error("Invalid metadata")]
    InvalidMetadataError
}
