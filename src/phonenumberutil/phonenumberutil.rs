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

//! This module provides the main entry point for interacting with the phone number handling library.
//!
//! It exposes the `PhoneNumberUtil` struct, which contains a comprehensive set of methods
//! for parsing, formatting, validating, and analyzing phone numbers from various regions
//! around the world. This utility is designed to handle the complexities of international
//! phone number formats, country codes, and numbering plans.

use std::borrow::Cow;

use crate::{
    generated::proto::phonenumber::PhoneNumber, 
};

use super::{
    errors::{ParseError, ValidationError, GetExampleNumberError},
    enums::{PhoneNumberFormat, PhoneNumberType, MatchType, NumberLengthType},
    phonenumberutil_internal::PhoneNumberUtilInternal,
};


/// The main struct for all phone number-related operations.
///
/// It encapsulates the library's core logic and provides a public API for parsing,
/// formatting, and validating phone numbers. An instance of this struct is the
/// primary entry point for using the library's features.
pub struct PhoneNumberUtil {
    util_internal: PhoneNumberUtilInternal
}

impl PhoneNumberUtil {
    
    /// Creates new `PhoneNumberUtil` instance
    pub fn new() -> Self {
        Self { util_internal: 
            PhoneNumberUtilInternal::new()
                .expect("Metadata should be valid and all regex should compile") 
        }
    }

    /// Checks if a `PhoneNumber` can be dialed internationally.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: A reference to the `PhoneNumber` object to be checked.
    ///
    /// # Returns
    ///
    /// `true` if the number can be dialed from another country, `false` otherwise.
    ///
    /// # Panics
    ///
    /// This method panics if the underlying metadata contains an invalid regular expression,
    /// which indicates a critical library bug.
    pub fn can_be_internationally_dialled(&self, phone_number: &PhoneNumber) -> bool {
        self.util_internal
            .can_be_internationally_dialled(phone_number)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Converts all alpha characters in a phone number string to their corresponding digits.
    ///
    /// For example, an input of "1-800-FLOWERS" will be converted to "1-800-3569377".
    ///
    
    /// # Parameters
    ///
    /// * `number`: A string slice or `String` representing the phone number.
    ///
    /// # Returns
    ///
    /// A `String` containing the phone number with all alphabetic characters converted to digits.
    pub fn convert_alpha_characters_in_number<'a>(&self, number: impl AsRef<str>) -> String {
        self.util_internal.convert_alpha_characters_in_number(number.as_ref())
    }

    /// Formats a `PhoneNumber` into a standardized format.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to be formatted.
    /// * `number_format`: The `PhoneNumberFormat` to be applied (e.g., E164, INTERNATIONAL, NATIONAL).
    ///
    /// # Returns
    ///
    /// A `Cow<'a, str>` which is either a borrowed reference to a pre-formatted string or a
    /// newly allocated `String` with the formatted number.
    ///
    /// # Panics
    ///
    /// This method panics if the underlying metadata contains an invalid regular expression,
    /// indicating a library bug.
    pub fn format<'a>(&self, phone_number: &'a PhoneNumber, number_format: PhoneNumberFormat) -> Cow<'a, str> {
        self.util_internal
            .format(phone_number, number_format)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Formats a `PhoneNumber`, attempting to preserve original formatting and punctuation.
    ///
    /// The number is formatted in the national format of the region it is from.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to be formatted.
    /// * `region_calling_from`: The two-letter region code (ISO 3166-1) from where the call is being made.
    ///
    /// # Returns
    ///
    /// A `Cow<'a, str>` containing the formatted number.
    ///
    /// # Panics
    ///
    /// This method panics if metadata is invalid, which indicates a library bug.
    pub fn format_in_original_format<'a>(
        &self, phone_number: &'a PhoneNumber, region_calling_from: impl AsRef<str>
    ) -> Cow<'a, str> {
        self.util_internal
            .format_in_original_format(phone_number, region_calling_from.as_ref())
            // This should not never happen
            .expect("A valid regex and region is expected in metadata; this indicates a library bug.")
    }

    /// Formats a national number with a specified carrier code.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to format.
    /// * `carrier_code`: The carrier code to prepend to the number.
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted number.
    ///
    /// # Panics
    ///
    /// Panics if metadata is invalid, indicating a library bug.
    pub fn format_national_number_with_carrier_code<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        carrier_code: impl AsRef<str>,
    ) -> String {
        self.util_internal
            .format_national_number_with_carrier_code(phone_number, carrier_code.as_ref())
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Formats a `PhoneNumber` for dialing from a mobile device.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to format.
    /// * `region_calling_from`: The two-letter region code (ISO 3166-1) where the user is.
    /// * `with_formatting`: If `true`, the number is formatted with punctuation; otherwise, only digits are returned.
    ///
    /// # Returns
    ///
    /// A `Cow<'a, str>` with the dialable number.
    ///
    /// # Panics
    ///
    /// Panics if formatting fails due to a library bug.
    pub fn format_number_for_mobile_dialing<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        region_calling_from: impl AsRef<str>,
        with_formatting: bool,
    ) -> Cow<'a, str> {
        self.util_internal
            .format_number_for_mobile_dialing(phone_number, region_calling_from.as_ref(), with_formatting)
            .expect("Formatting failed; this indicates a library bug.")
    }
    
    /// Formats a `PhoneNumber` for out-of-country calling.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to format.
    /// * `region_calling_from`: The two-letter region code (ISO 3166-1) of the calling location.
    ///
    /// # Returns
    ///
    /// A `Cow<'a, str>` representing the number formatted for international dialing.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn format_out_of_country_calling_number<'a>(
        &self, phone_number: &'a PhoneNumber, region_calling_from: impl AsRef<str>
    ) -> Cow<'a, str> {
        self.util_internal
            .format_out_of_country_calling_number(phone_number, region_calling_from.as_ref())
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Formats a `PhoneNumber` for out-of-country calling while preserving any alphabetic characters.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to format.
    /// * `region_calling_from`: The two-letter region code (ISO 3166-1) of the calling location.
    ///
    /// # Returns
    ///
    /// A `Cow<'a, str>` with the formatted number.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn format_out_of_country_keeping_alpha_chars<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        region_calling_from: impl AsRef<str>,
    ) -> Cow<'a, str> {
        self.util_internal
            .format_out_of_country_keeping_alpha_chars(phone_number, region_calling_from.as_ref())
            .expect("Formatting failed; this indicates a library bug.")
    }

    /// Retrieves the country calling code for a given region.
    ///
    /// # Parameters
    ///
    /// * `region_code`: The two-letter region code (ISO 3166-1).
    ///
    /// # Returns
    ///
    /// An `Option<i32>` containing the country code, or `None` if the region code is invalid.
    pub fn get_country_code_for_region(&self, region_code: impl AsRef<str>) -> Option<i32> {
        self.util_internal
            .get_country_code_for_region(region_code.as_ref())
    }

    /// Gets a valid example `PhoneNumber` for a specific region.
    ///
    /// # Parameters
    ///
    /// * `region_code`: The two-letter region code (ISO 3166-1).
    ///
    /// # Returns
    ///
    /// A `Result` containing a valid `PhoneNumber` on success, or a `GetExampleNumberError` on failure.
    pub fn get_example_number(&self, region_code: impl AsRef<str>) -> Result<PhoneNumber, GetExampleNumberError> {
        self.util_internal.get_example_number(region_code.as_ref())
            .map_err(|err| err.into_public())
    }

    /// Gets a valid example `PhoneNumber` for a specific number type.
    ///
    /// # Parameters
    ///
    /// * `number_type`: The desired `PhoneNumberType` (e.g., MOBILE, TOLL_FREE).
    ///
    /// # Returns
    ///
    /// A `Result` containing a `PhoneNumber` on success, or `GetExampleNumberError` if no example exists.
    pub fn get_example_number_for_type(
        &self,
        number_type: PhoneNumberType,
    ) -> Result<PhoneNumber, GetExampleNumberError> {
        self.util_internal.get_example_number_for_type(number_type)
            .map_err(|err| err.into_public())
    }

    /// Gets an invalid but plausible example `PhoneNumber` for a specific region.
    ///
    /// # Parameters
    ///
    /// * `region_code`: The two-letter region code (ISO 3166-1).
    ///
    /// # Returns
    ///
    /// A `Result` containing an invalid `PhoneNumber` on success, or a `GetExampleNumberError` on failure.
    pub fn get_invalid_example_number(&self, region_code: impl AsRef<str>) -> Result<PhoneNumber, GetExampleNumberError> {
        self.util_internal.get_invalid_example_number(region_code.as_ref())
            .map_err(|err| err.into_public())
    }

    /// Gets the length of the geographical area code from a `PhoneNumber`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to examine.
    ///
    /// # Returns
    ///
    /// The length of the area code, or `0` if it cannot be determined.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn get_length_of_geographical_area_code(&self, phone_number: &PhoneNumber) -> usize {
        self.util_internal
            .get_length_of_geographical_area_code(phone_number)
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Gets the length of the national destination code from a `PhoneNumber`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to examine.
    ///
    /// # Returns
    ///
    /// The length of the national destination code.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn get_length_of_national_destination_code(&self, phone_number: &PhoneNumber) -> usize {
        self.util_internal
            .get_length_of_national_destination_code(phone_number)
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Gets the National Significant Number (NSN) from a `PhoneNumber`.
    ///
    /// The NSN is the part of the number that follows the country code.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` from which to extract the NSN.
    ///
    /// # Returns
    ///
    /// A `String` containing the NSN.
    pub fn get_national_significant_number<'a>(&self, phone_number: &'a PhoneNumber) -> String {
        self.util_internal.get_national_significant_number(phone_number)
    }

    /// Determines the `PhoneNumberType` of a given `PhoneNumber`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to be categorized.
    ///
    /// # Returns
    ///
    /// The `PhoneNumberType` (e.g., MOBILE, FIXED_LINE, UNKNOWN).
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn get_number_type(&self, phone_number: &PhoneNumber) -> PhoneNumberType {
        self
            .util_internal
            .get_number_type(phone_number)
            // This should not never happen
            .expect("A valid regex and region is expected in metadata; this indicates a library bug.")
    }

    /// Gets the primary region code for a given country calling code.
    ///
    /// Note: Some country codes are shared by multiple regions (e.g., +1 for USA, Canada).
    /// This returns the main region for that code (e.g., "US" for +1).
    ///
    /// # Parameters
    ///
    /// * `country_code`: The country calling code.
    ///
    /// # Returns
    ///
    /// A string slice with the corresponding two-letter region code. Returns "ZZ" for invalid codes.
    pub fn get_region_code_for_country_code(&self, country_code: i32) -> &str {
        self.util_internal.get_region_code_for_country_code(country_code)
    }

    /// Gets the region code for a `PhoneNumber`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to identify.
    ///
    /// # Returns
    ///
    /// A string slice with the two-letter region code.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn get_region_code_for_number(&self, phone_number: &PhoneNumber) -> &str {
        self
            .util_internal
            .get_region_code_for_number(phone_number)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")            
    }
    
    /// Gets all region codes associated with a country calling code.
    ///
    /// # Parameters
    ///
    /// * `country_code`: The country calling code.
    ///
    /// # Returns
    ///
    /// An `Option` containing an iterator over all associated region codes, or `None` if the
    /// country code is invalid.
    pub fn get_region_codes_for_country_code(&self, country_code: i32) -> Option<impl ExactSizeIterator<Item=&str>> {
        self.util_internal.get_region_codes_for_country_calling_code(country_code)
    }

    /// Gets an iterator over all supported two-letter region codes.
    ///
    /// # Returns
    ///
    /// An `ExactSizeIterator` that yields string slices of all supported region codes.
    pub fn get_supported_regions(&self) -> impl ExactSizeIterator<Item=&str> {
        self.util_internal.get_supported_regions()
    }

    /// Checks if a number string contains alphabetic characters.
    ///
    /// # Parameters
    ///
    /// * `number`: The phone number string to check.
    ///
    /// # Returns
    ///
    /// `true` if the string contains letters, `false` otherwise.
    pub fn is_alpha_number(&self, number: impl AsRef<str>) -> bool {
        self.util_internal.is_alpha_number(number.as_ref())
    }

    /// Checks if a region is part of the North American Numbering Plan (NANPA).
    ///
    /// # Parameters
    ///
    /// * `region_code`: The two-letter region code (ISO 3166-1) to check.
    ///
    /// # Returns
    ///
    /// `true` if the region is a NANPA country, `false` otherwise.
    pub fn is_nanpa_country(&self, region_code: impl AsRef<str>) -> bool {
        self.util_internal.is_nanpa_country(region_code.as_ref())
    }

    /// Checks if a `PhoneNumber` is geographical.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to check.
    ///
    /// # Returns
    ///
    /// `true` if the number corresponds to a specific geographic area.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn is_number_geographical(&self, phone_number: &PhoneNumber) -> bool {
        self.util_internal.is_number_geographical(phone_number)
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Compares two phone numbers and returns their `MatchType`.
    ///
    /// # Parameters
    ///
    /// * `first_number`: The first `PhoneNumber` to compare.
    /// * `second_number`: The second `PhoneNumber` to compare.
    ///
    /// # Returns
    ///
    /// The `MatchType` indicating the level of similarity (e.g., EXACT_MATCH, NSN_MATCH).
    pub fn is_number_match(
        &self,
        first_number: &PhoneNumber,
        second_number: &PhoneNumber,
    ) -> MatchType {
        self.util_internal
            .is_number_match(first_number, second_number)
    }

    /// Performs a fast check to determine if a `PhoneNumber` is possibly valid.
    ///
    /// This method is less strict than `is_valid_number`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to check.
    ///
    /// # Returns
    ///
    /// `true` if the number has a valid length, `false` otherwise.
    pub fn is_possible_number(&self, phone_number: &PhoneNumber) -> bool {
        self.util_internal.is_possible_number(phone_number)
    }

    /// Checks if a `PhoneNumber` is possibly valid and provides a reason if not.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to check.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(NumberLengthType)` on success or a `ValidationError` on failure.
    pub fn is_possible_number_with_reason(&self, phone_number: &PhoneNumber) -> Result<NumberLengthType, ValidationError> {
        self.util_internal.is_possible_number_with_reason(phone_number)
    }

    /// Performs a full validation of a `PhoneNumber`.
    ///
    /// This is a more comprehensive check than `is_possible_number`.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to validate.
    ///
    /// # Returns
    ///
    /// `true` if the number is valid, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn is_valid_number(&self, phone_number: &PhoneNumber) -> bool {
        self
            .util_internal
            .is_valid_number(phone_number)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }

    /// Validates a `PhoneNumber` for a specific region.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: The `PhoneNumber` to validate.
    /// * `region`: The two-letter region code (ISO 3166-1) to validate against.
    ///
    /// # Returns
    ///
    /// `true` if the number is valid for the given region, `false` otherwise.
    pub fn is_valid_number_for_region(&self, phone_number: &PhoneNumber, region: impl AsRef<str>) -> bool {
        self.util_internal.is_valid_number_for_region(phone_number, region.as_ref())
    }

    /// Parses a string into a `PhoneNumber`, keeping the raw input string.
    ///
    /// # Parameters
    ///
    /// * `number_to_parse`: The phone number string.
    /// * `default_region`: The two-letter region code (ISO 3166-1) to use if the number is not in international format.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `PhoneNumber` on success, or a `ParseError` on failure.
    pub fn parse_and_keep_raw_input(
        &self,
        number_to_parse: impl AsRef<str>,
        default_region: impl AsRef<str>,
    ) -> Result<PhoneNumber, ParseError> {
        self.util_internal
            .parse_and_keep_raw_input(number_to_parse.as_ref(), default_region.as_ref())
            .map_err(| err | err.into_public())
    }

    /// Parses a string into a `PhoneNumber`.
    ///
    /// This is the primary method for converting a string representation of a number
    /// into a structured `PhoneNumber` object.
    ///
    /// # Parameters
    ///
    /// * `number_to_parse`: The phone number string.
    /// * `default_region`: The two-letter region code (ISO 3166-1) to use if the number is not in international format.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `PhoneNumber` on success, or a `ParseError` on failure.
    pub fn parse(
        &self,
        number_to_parse: impl AsRef<str>,
        default_region: impl AsRef<str>,
    ) -> Result<PhoneNumber, ParseError> {
        self.util_internal
            .parse(number_to_parse.as_ref(), default_region.as_ref())
            .map_err(| err | err.into_public())
    }

    /// Truncates a `PhoneNumber` that is too long to a valid length.
    ///
    /// # Parameters
    ///
    /// * `phone_number`: A mutable reference to the `PhoneNumber` to truncate.
    ///
    /// # Returns
    ///
    /// `true` if the number was truncated, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Panics on invalid metadata, indicating a library bug.
    pub fn truncate_too_long_number(&self, phone_number: &mut PhoneNumber) -> bool {
        self.util_internal.truncate_too_long_number(phone_number)
            // This should not never happen
            .expect("A valid regex is expected in metadata; this indicates a library bug.")
    }
}

