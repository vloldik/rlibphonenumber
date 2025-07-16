// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 Kashin Vladislav (Rust adaptation author)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::num::ParseIntError;

use thiserror::Error;

use crate::regexp_cache::InvalidRegexError;

#[derive(Debug, PartialEq, Error)]
pub enum InternalLogicError {
    #[error("{0}")]
    InvalidRegex(#[from] InvalidRegexError),
    
    #[error("{0}")]
    InvalidMetadataForValidRegion(#[from] InvalidMetadataForValidRegionError)
}   

#[derive(Debug, PartialEq, Error)]
pub enum ParseErrorInternal {
    #[error("{0}")]
    FailedToParse(#[from] ParseError),
    #[error("{0}")]
    RegexError(#[from] InvalidRegexError)
}

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    // Removed as OK variant
    // NoParsingError,
    #[error("Invalid country code")]
    InvalidCountryCode, // INVALID_COUNTRY_CODE in the java version.
    #[error("Not a number: {0}")]
    NotANumber(#[from] NotANumberError),
    #[error("Too short after idd")]
    TooShortAfterIdd,
    #[error("Too short Nsn")]
    TooShortNsn,
    #[error("Too long nsn")]
    TooLongNsn, // TOO_LONG in the java version.
}

#[derive(Debug, PartialEq, Error)]
pub enum NotANumberError {
    #[error("Number not matched a valid number pattern")]
    NotMatchedValidNumberPattern,
    #[error("Invalid phone context")]
    InvalidPhoneContext,
    #[error("{0}")]
    FailedToParseNumberAsInt(#[from] ParseIntError),
    #[error("{0}")]
    FailedToExtractNumber(#[from] ExtractNumberError),
}

#[derive(Debug, PartialEq, Error)]
pub enum ExtractNumberError {
    #[error("No valid start character found")]
    NoValidStartCharacter,
    #[error("Invalid number")]
    NotANumber,
}


#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberErrorInternal {
    #[error("{0}")]
    FailedToGetExampleNumber(#[from] GetExampleNumberError),
    #[error("{0}")]
    RegexError(#[from] InvalidRegexError)
}

#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberError {
    #[error("Parse error: {0}")]
    FailedToParse(#[from] ParseError),
    #[error("No example number")]
    NoExampleNumber,
    #[error("Could not get number")]
    CouldNotGetNumber,
    #[error("Invalid country code provided")]
    InvalidRegionCode
}

#[derive(Debug, PartialEq, Error)]
pub enum InvalidNumberErrorInternal {
    #[error("{0}")]
    InvalidNumber(#[from] InvalidNumberError),
    #[error("{0}")]
    InvalidRegex(#[from] InvalidRegexError)
}

#[derive(Error, Debug, PartialEq)]
#[error("Invalid number given")]
pub struct InvalidNumberError(#[from] pub ParseError);  // NOT_A_NUMBER in the java version

#[derive(Debug, Error, PartialEq)]
#[error("Metadata for valid region MUST not be null")]
pub struct InvalidMetadataForValidRegionError;

/// Possible outcomes when testing if a PhoneNumber is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum ValidationError {
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

impl From<ParseErrorInternal> for GetExampleNumberErrorInternal {
    fn from(value: ParseErrorInternal) -> Self {
        match value {
            ParseErrorInternal::FailedToParse(err) => 
                GetExampleNumberError::FailedToParse(err).into(),
            ParseErrorInternal::RegexError(err) =>
                GetExampleNumberErrorInternal::RegexError(err)
        }
    }
}

impl From<ParseErrorInternal> for InvalidNumberErrorInternal {
    fn from(value: ParseErrorInternal) -> Self {
        match value {
            ParseErrorInternal::FailedToParse(err) => 
                InvalidNumberError(err).into(),
            ParseErrorInternal::RegexError(err) =>
                InvalidNumberErrorInternal::InvalidRegex(err)
        }
    }
}

impl From<ExtractNumberError> for ParseError {
    fn from(value: ExtractNumberError) -> Self {
        NotANumberError::FailedToExtractNumber(value).into()
    }
}

impl GetExampleNumberErrorInternal {
    pub fn into_public(self) -> GetExampleNumberError {
        match self {
            GetExampleNumberErrorInternal::FailedToGetExampleNumber(err) => err,
            GetExampleNumberErrorInternal::RegexError(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}

impl ParseErrorInternal {
    pub fn into_public(self) -> ParseError {
        match self {
            ParseErrorInternal::FailedToParse(err) => err,
            ParseErrorInternal::RegexError(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}

impl InvalidNumberErrorInternal {
    pub fn into_public(self) -> InvalidNumberError {
        match self {
            InvalidNumberErrorInternal::InvalidNumber(err) => err,
            InvalidNumberErrorInternal::InvalidRegex(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}

