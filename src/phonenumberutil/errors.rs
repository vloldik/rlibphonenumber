use std::num::ParseIntError;

use thiserror::Error;

use crate::regexp_cache::ErrorInvalidRegex;

#[derive(Debug, PartialEq, Error)]
pub enum InternalLogicError {
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex),
    
    #[error("{0}")]
    InvalidMetadataForValidRegionError(#[from] InvalidMetadataForValidRegionError)
}   

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    // Removed as OK variant
    // NoParsingError,
    #[error("Invalid country code")]
    InvalidCountryCodeError, // INVALID_COUNTRY_CODE in the java version.
    #[error("Not a number: {0}")]
    NotANumber(#[from] NotANumberError),
    #[error("Too short after idd")]
    TooShortAfterIdd,
    #[error("Too short Nsn")]
    TooShortNsn,
    #[error("Too long nsn")]
    TooLongNsn, // TOO_LONG in the java version.
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex),
}

#[derive(Debug, PartialEq, Error)]
pub enum NotANumberError {
    #[error("Number not matched a valid number pattern")]
    NotMatchedValidNumberPattern,
    #[error("Invalid phone context")]
    InvalidPhoneContext,
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

impl From<ExtractNumberError> for ParseError {
    fn from(value: ExtractNumberError) -> Self {
        NotANumberError::ExtractNumberError(value).into()
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberError {
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
    #[error("{0}")]
    InternalLogicError(#[from] InternalLogicError),
    #[error("No example number")]
    NoExampleNumberError,
    #[error("Could not get number")]
    CouldNotGetNumberError,
    #[error("Invalid metadata")]
    InvalidMetadataError
}


#[derive(Error, Debug, PartialEq)]
#[error("Invalid number given")]
pub struct InvalidNumberError(#[from] pub ParseError);  // NOT_A_NUMBER in the java version

#[derive(Debug, Error, PartialEq)]
#[error("Metadata for valid region MUST not be null")]
pub struct InvalidMetadataForValidRegionError;

/// Possible outcomes when testing if a PhoneNumber is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum ValidationResultErr {
    /// The number has an invalid country calling code.
    #[error("The number has an invalid country calling code")]
    InvalidCountryCode,
    /// The number is shorter than all valid numbers for this region.
    #[error("The number is shorter than all valid numbers for this region")]
    TooShort,
    /// The number is longer than the shortest valid numbers for this region,
    /// shorter than the longest valid numbers for this region, and does not
    /// itself have a number length that matches valid numbers for this region.
    /// This can also be returned in the case where
    /// IsPossibleNumberForTypeWithReason was called, and there are no numbers of
    /// this type at all for this region.
    #[error("\
    The number is longer than the shortest valid numbers for this region,\
    shorter than the longest valid numbers for this region, and does not\
    itself have a number length that matches valid numbers for this region\
    ")]
    InvalidLength,
    /// The number is longer than all valid numbers for this region.
    #[error("The number is longer than all valid numbers for this region")]
    TooLong,
}