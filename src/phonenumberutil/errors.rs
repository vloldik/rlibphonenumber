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

/// Represents critical internal errors that indicate a bug within the library itself.
/// These errors are not expected to be caught or handled by the user, as they
/// signal a problem with the library's metadata or logic.
#[derive(Debug, PartialEq, Error)]
pub enum InternalLogicError {
    /// An error indicating that a regular expression provided in the metadata is invalid.
    /// This points to a bug in the library's bundled metadata files.
    #[error("{0}")]
    InvalidRegex(#[from] InvalidRegexError),
    
    /// An error indicating that metadata for a valid, supported region is unexpectedly missing.
    /// This points to a bug in the library's initialization or metadata loading.
    #[error("{0}")]
    InvalidMetadataForValidRegion(#[from] InvalidMetadataForValidRegionError)
}   

/// An internal error type used during the parsing process.
/// It distinguishes between a general parsing failure and a regex-specific issue.
#[derive(Debug, PartialEq, Error)]
pub enum ParseErrorInternal {
    /// Wraps a public `ParseError`, representing a standard parsing failure.
    #[error("{0}")]
    FailedToParse(#[from] ParseError),
    /// An error indicating that a regular expression was invalid during parsing.
    /// This signals a bug in the library's metadata.
    #[error("{0}")]
    RegexError(#[from] InvalidRegexError)
}

/// Represents the possible errors that can occur when parsing a phone number string.
/// This is a public-facing error enum.
#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    /// **Invalid country code.**
    /// This error occurs if the number begins with a `+` but is followed by an
    /// invalid or unrecognized country calling code.
    #[error("Invalid country code")]
    InvalidCountryCode,
    /// **The string is not a number.**
    /// The input string contains invalid characters or does not conform to a recognizable
    /// phone number format. This variant wraps a `NotANumberError` for more detail.
    #[error("Not a number: {0}")]
    NotANumber(#[from] NotANumberError),
    /// **The number is too short after the International Direct Dialing (IDD) code.**
    /// After stripping a valid IDD prefix, the remaining part of the number is too
    /// short to be a valid national number.
    #[error("Too short after idd")]
    TooShortAfterIdd,
    /// **The National Significant Number (NSN) is too short.**
    /// The number, after stripping the country code and any carrier codes, is shorter
    /// than any possible valid number for that region.
    #[error("Too short Nsn")]
    TooShortNsn,
    /// **The National Significant Number (NSN) is too long.**
    /// The number, after stripping the country code, is longer than any possible
    /// valid number for that region.
    #[error("Too long nsn")]
    TooLongNsn,
}

/// Provides more specific details for a `ParseError::NotANumber` failure.
#[derive(Debug, PartialEq, Error)]
pub enum NotANumberError {
    /// The number string does not match the basic regular expression for a valid
    /// phone number pattern.
    #[error("Number not matched a valid number pattern")]
    NotMatchedValidNumberPattern,
    /// The phone number context is invalid, such as an incorrect "tel:" prefix.
    #[error("Invalid phone context")]
    InvalidPhoneContext,
    /// A numeric portion of the phone number string could not be parsed into an integer.
    #[error("{0}")]
    FailedToParseNumberAsInt(#[from] ParseIntError),
    /// An error occurred at the lowest level of extracting a numeric string from the input.
    #[error("{0}")]
    FailedToExtractNumber(#[from] ExtractNumberError),
}

/// Represents errors during the low-level extraction of a number string.
#[derive(Debug, PartialEq, Error)]
pub enum ExtractNumberError {
    /// The input string does not contain a character that could begin a phone number
    /// (e.g., a digit, `+`, or `#`).
    #[error("No valid start character found")]
    NoValidStartCharacter,
    /// Number did not match valid number pattern.
    #[error("Invalid number")]
    NotANumber,
}


/// Internal error type used when fetching an example number.
#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberErrorInternal {
    /// Wraps a public `GetExampleNumberError` for standard failures.
    #[error("{0}")]
    FailedToGetExampleNumber(#[from] GetExampleNumberError),
    /// An error indicating that a regular expression was invalid while generating the example.
    /// This signals a bug in the library's metadata.
    #[error("{0}")]
    RegexError(#[from] InvalidRegexError)
}

/// Represents possible failures when requesting an example phone number.
#[derive(Debug, PartialEq, Error)]
pub enum GetExampleNumberError {
    /// An internal parsing error occurred while constructing the example number.
    #[error("Parse error: {0}")]
    FailedToParse(#[from] ParseError),
    /// No example number is available for the requested region or number type in the metadata.
    #[error("No example number")]
    NoExampleNumber,
    /// A generic failure occurred while trying to retrieve the number.
    #[error("Could not get number")]
    CouldNotGetNumber,
    /// The provided region code is invalid or not supported by the library.
    #[error("Invalid country code provided")]
    InvalidRegionCode
}

/// Internal error type used during number validation.
#[derive(Debug, PartialEq, Error)]
pub enum InvalidNumberErrorInternal {
    /// Wraps a public `InvalidNumberError`.
    #[error("{0}")]
    InvalidNumber(#[from] InvalidNumberError),
    /// An error indicating a regex was invalid during validation, signaling a library bug.
    #[error("{0}")]
    InvalidRegex(#[from] InvalidRegexError)
}

/// A specific error indicating that the provided input is not a number.
///
/// This is typically returned when a check requires a valid number, but parsing fails.
#[derive(Error, Debug, PartialEq)]
#[error("Invalid number given")]
pub struct InvalidNumberError(#[from] pub ParseError);

/// An internal error indicating that metadata for a supposedly valid region was `null`.
///
/// This represents a critical bug in the library's metadata loading or structure,
/// as a supported region should always have associated metadata.
#[derive(Debug, Error, PartialEq)]
#[error("Metadata for valid region MUST not be null")]
pub struct InvalidMetadataForValidRegionError;

/// Details why a phone number is considered invalid.
///
/// This enum is returned by validation functions to provide a specific reason
/// for the failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum ValidationError {
    /// **The country calling code is invalid.**
    /// The number has a country code that does not correspond to any known region.
    #[error("The number has an invalid country calling code")]
    InvalidCountryCode,
    /// **The number is too short.**
    /// The number's length is shorter than the shortest possible valid number
    /// for its region.
    #[error("The number is shorter than all valid numbers for this region")]
    TooShort,
    /// **The number has an invalid length.**
    /// The number's length falls between the shortest and longest possible lengths
    /// for its region but does not match any specific valid length. This can also occur
    ///  if no numbers of the requested type exist for the region.
    #[error("\
    The number is longer than the shortest valid numbers for this region,\
    shorter than the longest valid numbers for this region, and does not\
    itself have a number length that matches valid numbers for this region\
    ")]
    InvalidLength,
    /// **The number is too long.**
    /// The number's length is longer than the longest possible valid number
    /// for its region.
    #[error("The number is longer than all valid numbers for this region")]
    TooLong,
}

impl From<ParseErrorInternal> for GetExampleNumberErrorInternal {
    /// Converts an internal parsing error into an internal "get example number" error.
    /// This is used to propagate errors within the library's logic.
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
    /// Converts an internal parsing error into an internal "invalid number" error.
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
    /// Converts a low-level `ExtractNumberError` into a public-facing `ParseError`.
    /// This simplifies the error API by nesting specific errors within more general ones.
    fn from(value: ExtractNumberError) -> Self {
        NotANumberError::FailedToExtractNumber(value).into()
    }
}

impl GetExampleNumberErrorInternal {
    /// Converts an internal error into its public-facing equivalent.
    ///
    /// If the error is a `RegexError`, this method will panic, as this indicates a
    /// non-recoverable library bug that should be fixed.
    pub fn into_public(self) -> GetExampleNumberError {
        match self {
            GetExampleNumberErrorInternal::FailedToGetExampleNumber(err) => err,
            GetExampleNumberErrorInternal::RegexError(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}

impl ParseErrorInternal {
    /// Converts an internal parsing error into its public-facing `ParseError`.
    ///
    /// If the error is a `RegexError`, this method will panic, enforcing that the
    /// library's metadata must be valid.
    pub fn into_public(self) -> ParseError {
        match self {
            ParseErrorInternal::FailedToParse(err) => err,
            ParseErrorInternal::RegexError(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}

impl InvalidNumberErrorInternal {
    /// Converts an internal validation error into its public `InvalidNumberError`.
    ///
    /// If the error is a `RegexError`, this method will panic, as it signifies a
    /// critical library bug.
    pub fn into_public(self) -> InvalidNumberError {
        match self {
            InvalidNumberErrorInternal::InvalidNumber(err) => err,
            InvalidNumberErrorInternal::InvalidRegex(err) => 
                panic!("A valid regex is expected in metadata; this indicates a library bug! {}", err)
        }
    }
}