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

use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    num::ParseIntError,
};

use prost::DecodeError;
use thiserror::Error;

use crate::InvalidRegionError;

#[derive(Debug, PartialEq, Error, Clone)]
#[error("An error occurred while trying to create regex: {0}")]
pub struct InvalidRegexError(#[from] crate::regexp::Error);

/// Represents the possible errors that can occur when parsing a phone number string.
/// This is a public-facing error enum.
#[derive(Debug, PartialEq, Clone, Error)]
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
#[derive(Debug, PartialEq, Clone, Error)]
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
#[derive(Debug, PartialEq, Clone, Error)]
pub enum ExtractNumberError {
    /// The input string does not contain a character that could begin a phone number
    /// (e.g., a digit, `+`, or `#`).
    #[error("No valid start character found")]
    NoValidStartCharacter,
    /// Number did not match valid number pattern.
    #[error("Invalid number")]
    NotANumber,
}

#[derive(Debug, PartialEq, Clone, Error)]
pub enum InternalError<T: Debug + Display> {
    #[error("Wrapped error: {0}")]
    Wrapped(T),
    /// An error indicating that a regular expression was invalid while generating the example.
    /// This signals a bug in the library's metadata.
    #[error("{0}")]
    RegexError(#[from] InvalidRegexError),
}

pub type InternalRegexError = InternalError<Infallible>;

/// Represents possible failures when requesting an example phone number.
#[derive(Debug, PartialEq, Clone, Error)]
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
    InvalidRegionCode,
}

/// A specific error indicating that the provided input is not a number.
///
/// This is typically returned when a check requires a valid number, but parsing fails.
#[derive(Error, Debug, Clone, PartialEq)]
#[error("Invalid number given")]
pub struct InvalidNumberError(#[from] pub ParseError);

/// An internal error indicating that metadata for a supposedly valid region was `null`.
///
/// This represents a critical bug in the library's metadata loading or structure,
/// as a supported region should always have associated metadata.
#[derive(Debug, Error, Clone, PartialEq)]
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
    #[error(
        "\
    The number is longer than the shortest valid numbers for this region,\
    shorter than the longest valid numbers for this region, and does not\
    itself have a number length that matches valid numbers for this region\
    "
    )]
    InvalidLength,
    /// **The number is too long.**
    /// The number's length is longer than the longest possible valid number
    /// for its region.
    #[error("The number is longer than all valid numbers for this region")]
    TooLong,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CreateUtilError {
    #[error("Failed to create region: region should be valid 2-digit ascii string, got: {0}")]
    InvalidRegion(#[from] InvalidRegionError),

    #[error("Failed to decode metadata from static file, this indicates library bug: {0}")]
    Decode(#[from] DecodeError),
}

pub fn unwrap_internal<T, E: Debug + Display>(result: Result<T, InternalError<E>>) -> Result<T, E> {
    match result {
        Err(InternalError::Wrapped(err)) => Err(err),
        Err(InternalError::RegexError(invalid_regex_error)) => panic!(
            "A valid regex is expected in metadata; this indicates a library bug: {}",
            invalid_regex_error
        ),
        Ok(result) => Ok(result),
    }
}

pub fn unwrap_internal_infallible<T>(result: Result<T, InternalError<Infallible>>) -> T {
    unwrap_internal(result).unwrap_or_else(|err| match err {})
}

macro_rules! delegate_from {
    ($inner:ty : $($t:ty),*) => {
        impl From<$inner> for InternalError<$inner> {
            fn from(value: $inner) -> Self {
                Self::Wrapped(value)
            }
        }
        $(
            impl From<$t> for InternalError<$inner> {
                fn from(value: $t) -> Self {
                    InternalError::from(<$inner>::from(value))
                }
            }
        )*
    };
}

delegate_from!(GetExampleNumberError: ParseError);
delegate_from!(ParseError: NotANumberError);
delegate_from!(NotANumberError: ExtractNumberError);

impl<T: Debug + Display> InternalError<T> {
    pub fn translate<O: From<T> + Debug + Display>(self) -> InternalError<O> {
        match self {
            Self::RegexError(e) => InternalError::RegexError(e),
            Self::Wrapped(e) => InternalError::Wrapped(e.into()),
        }
    }
}

impl InternalError<Infallible> {
    pub fn translate_internal<O: Debug + Display>(self) -> InternalError<O> {
        match self {
            InternalError::RegexError(invalid_regex_error) => {
                InternalError::RegexError(invalid_regex_error)
            }
        }
    }
}
