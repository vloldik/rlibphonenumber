// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 The Kashin Vladislav (Rust adaptation author)
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
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use super::phone_number_regexps_and_mappings::PhoneNumberRegExpsAndMappings;
use crate::{
    errors::NotANumberError, region_code::RegionCode, interfaces::MatcherApi, macros::owned_from_cow_or, phonemetadata::PhoneMetadataCollection, phonenumberutil::{
        errors::{
            ExtractNumberError, GetExampleNumberError, InternalLogicError,
            InvalidMetadataForValidRegionError, InvalidNumberError, ParseError,
            ValidationError,
        }, helper_constants::{
            DEFAULT_EXTN_PREFIX, MAX_LENGTH_COUNTRY_CODE, MAX_LENGTH_FOR_NSN, MIN_LENGTH_FOR_NSN,
            NANPA_COUNTRY_CODE, PLUS_SIGN, REGION_CODE_FOR_NON_GEO_ENTITY, RFC3966_EXTN_PREFIX,
            RFC3966_ISDN_SUBADDRESS, RFC3966_PHONE_CONTEXT, RFC3966_PREFIX,
        }, helper_functions::{
            self, copy_core_fields_only, get_number_desc_by_type, get_supported_types_for_metadata,
            is_national_number_suffix_of_the_other, load_compiled_metadata, normalize_helper,
            prefix_number_with_country_calling_code, test_number_length,
            test_number_length_with_unknown_type,
        }, helper_types::{PhoneNumberWithCountryCodeSource}, MatchType, PhoneNumberFormat, PhoneNumberType, NumberLengthType
    }, 
    phonemetadata::{NumberFormat, PhoneMetadata, PhoneNumberDesc},
    phonenumber::{phone_number::CountryCodeSource, PhoneNumber},
    regex_based_matcher::RegexBasedMatcher, regex_util::{RegexConsume, RegexFullMatch}, regexp_cache::ErrorInvalidRegex, string_util::strip_cow_prefix
};

use dec_from_char::DecimalExtended;
use log::{error, trace, warn};
use regex::Regex;

// Helper type for Result

pub type RegexResult<T> = std::result::Result<T, ErrorInvalidRegex>;

pub type ParseResult<T> = std::result::Result<T, ParseError>;

pub type ExampleNumberResult = std::result::Result<PhoneNumber, GetExampleNumberError>;
pub type ValidationResult = std::result::Result<NumberLengthType, ValidationError>;
pub type MatchResult = std::result::Result<MatchType, InvalidNumberError>;
pub type ExtractNumberResult<T> = std::result::Result<T, ExtractNumberError>;
pub type InternalLogicResult<T> = std::result::Result<T, InternalLogicError>;

pub struct PhoneNumberUtil {
    /// An API for validation checking.
    matcher_api: Box<dyn MatcherApi>,

    /// Helper class holding useful regular expressions and character mappings.
    reg_exps: PhoneNumberRegExpsAndMappings,

    /// A mapping from a country calling code to a RegionCode object which denotes
    /// NANPA share the country calling code 1 and Russia and Kazakhstan share the
    /// country calling code 7. Under this map, 1 is mapped to region code "US" and
    /// 7 is mapped to region code "RU". This is implemented as a sorted vector to
    /// achieve better performance.
    country_calling_code_to_region_code_map: Vec<(i32, Vec<String>)>,

    nanpa_regions: HashSet<String>,

    /// A mapping from a region code to a PhoneMetadata for that region.
    region_to_metadata_map: HashMap<String, PhoneMetadata>,

    /// A mapping from a country calling code for a non-geographical entity to the
    /// PhoneMetadata for that country calling code. Examples of the country
    /// calling codes include 800 (International Toll Free Service) and 808
    /// (International Shared Cost Service).
    country_code_to_non_geographical_metadata_map: HashMap<i32, PhoneMetadata>,
}

impl PhoneNumberUtil {
    pub(crate) fn new_for_metadata(metadata_collection: PhoneMetadataCollection) -> Self {
        let mut instance = Self {
            matcher_api: Box::new(RegexBasedMatcher::new()),
            reg_exps: PhoneNumberRegExpsAndMappings::new(),
            country_calling_code_to_region_code_map: Default::default(),
            nanpa_regions: Default::default(),
            region_to_metadata_map: Default::default(),
            country_code_to_non_geographical_metadata_map: Default::default(),
        };

        // that share a country calling code when inserting data.
        let mut country_calling_code_to_region_map = HashMap::<i32, VecDeque<String>>::new();
        for metadata in metadata_collection.metadata {
            let region_code = &metadata.id().to_string();
            let main_country_code = metadata.main_country_for_code();
            if RegionCode::get_unknown() == region_code {
                continue;
            }

            let country_calling_code = metadata.country_code();
            if REGION_CODE_FOR_NON_GEO_ENTITY == region_code {
                instance
                    .country_code_to_non_geographical_metadata_map
                    .insert(country_calling_code, metadata);
            } else {
                instance
                    .region_to_metadata_map
                    .insert(region_code.to_owned(), metadata);
            }

            let calling_code_in_map_o =
                country_calling_code_to_region_map.get_mut(&country_calling_code);
            if let Some(calling_code_in) = calling_code_in_map_o {
                if main_country_code {
                    calling_code_in.push_front(region_code.to_owned());
                } else {
                    calling_code_in.push_back(region_code.to_owned());
                }
            } else {
                // For most country calling codes, there will be only one region code.
                let mut list_with_region_code = VecDeque::new();
                list_with_region_code.push_back(region_code.to_owned());
                country_calling_code_to_region_map
                    .insert(country_calling_code, list_with_region_code);
            }
            if country_calling_code == NANPA_COUNTRY_CODE {
                instance.nanpa_regions.insert(region_code.to_owned());
            }
        }

        instance.country_calling_code_to_region_code_map.extend(
            country_calling_code_to_region_map
                .into_iter()
                .map(|(k, v)| (k, Vec::from(v))),
        );
        // Sort all the pairs in ascending order according to country calling code.
        instance
            .country_calling_code_to_region_code_map
            .sort_by_key(|(a, _)| *a);
        instance
    }

    pub(crate) fn new() -> Self {
        let metadata_collection = match load_compiled_metadata() {
            Err(err) => {
                let err_message = format!("Could not parse compiled-in metadata: {:?}", err);
                log::error!("{}", err_message);
                panic!("{}", err_message);
            }
            Ok(metadata) => metadata,
        };
        Self::new_for_metadata(metadata_collection)
    }

    pub fn get_supported_regions(&self) -> impl Iterator<Item = &str> {
        self.region_to_metadata_map.keys().map(|k| k.as_str())
    }

    pub fn get_supported_global_network_calling_codes(&self) -> impl Iterator<Item = i32> {
        self.country_code_to_non_geographical_metadata_map
            .keys()
            .map(|k| *k)
    }

    pub fn get_supported_calling_codes(&self) -> impl Iterator<Item = i32> {
        self.country_calling_code_to_region_code_map
            .iter()
            .map(|(k, _)| *k)
    }

    pub fn get_supported_types_for_region(
        &self,
        region_code: &str,
    ) -> Option<HashSet<PhoneNumberType>> {
        self.region_to_metadata_map
            .get(region_code)
            .and_then(|metadata| Some(get_supported_types_for_metadata(metadata)))
            .or_else(|| {
                warn!("Invalid or unknown region code provided: {}", region_code);
                None
            })
    }

    pub fn get_supported_types_for_non_geo_entity(
        &self,
        country_calling_code: i32,
    ) -> Option<HashSet<PhoneNumberType>> {
        self.country_code_to_non_geographical_metadata_map
            .get(&country_calling_code)
            .and_then(|metadata| Some(get_supported_types_for_metadata(metadata)))
            .or_else(|| {
                warn!(
                    "Unknown country calling code for a non-geographical entity provided: {}",
                    country_calling_code
                );
                None
            })
    }

    pub fn get_extn_patterns_for_matching(&self) -> &str {
        return &self.reg_exps.extn_patterns_for_matching;
    }

    pub fn starts_with_plus_chars_pattern(&self, phone_number: &str) -> bool {
        self.reg_exps.plus_chars_pattern.matches_start(phone_number)
    }

    pub fn contains_only_valid_digits(&self, s: &str) -> bool {
        self.reg_exps.digits_pattern.full_match(s)
    }

    pub fn trim_unwanted_end_chars<'a>(&self, phone_number: &'a str) -> &'a str {
        let mut bytes_to_trim = 0;

        for char in phone_number.chars().rev() {
            if !self
                .reg_exps
                .unwanted_end_char_pattern
                .full_match(&char.to_string())
            {
                break;
            }
            bytes_to_trim += char.len_utf8();
        }

        if bytes_to_trim > 0 {
            let new_len = phone_number.len() - bytes_to_trim;
            &phone_number[..new_len]
        } else {
            phone_number
        }
    }

    pub fn is_format_eligible_for_as_you_type_formatter(&self, format: &str) -> bool {
        // We require that the first
        // group is present in the output pattern to ensure no data is lost while
        // formatting; when we format as you type, this should always be the case.
        return self
            .reg_exps
            .is_format_eligible_as_you_type_formatting_regex
            .full_match(format);
    }

    pub fn formatting_rule_has_first_group_only(
        &self,
        national_prefix_formatting_rule: &str,
    ) -> bool {
        return national_prefix_formatting_rule.is_empty()
            || self
                .reg_exps
                .formatting_rule_has_first_group_only_regex
                .full_match(national_prefix_formatting_rule);
    }

    pub fn get_ndd_prefix_for_region(&self, region_code: &str, strip_non_digits: bool) -> Option<String> {
        self.region_to_metadata_map
            .get(region_code)
            .map(|metadata| {
                let mut prefix = metadata.national_prefix().to_owned();
                if strip_non_digits {
                    prefix = prefix.replace("~", "");
                }
                prefix
            })
    }

    /// 'hot' function wrapper for region_to_metadata_map.get
    pub fn get_metadata_for_region(&self, region_code: &str) -> Option<&PhoneMetadata> {
        return self.region_to_metadata_map.get(region_code);
    }

    pub fn format<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        number_format: PhoneNumberFormat,
    ) -> RegexResult<Cow<'b, str>> {
        if phone_number.national_number() == 0 {
            let raw_input = phone_number.raw_input();
            if !raw_input.is_empty() {
                // Unparseable numbers that kept their raw input just use that.
                // This is the only case where a number can be formatted as E164 without a
                // leading '+' symbol (but the original number wasn't parseable anyway).
                // TODO: Consider removing the 'if' above so that unparseable
                // strings without raw input format to the empty string instead of "+00".
                return Ok(Cow::Borrowed(raw_input));
            }
        }
        let country_calling_code = phone_number.country_code();
        let mut formatted_number = self.get_national_significant_number(phone_number);

        if matches!(number_format, PhoneNumberFormat::E164) {
            // Early exit for E164 case (even if the country calling code is invalid)
            // since no formatting of the national number needs to be applied.
            // Extensions are not formatted.
            prefix_number_with_country_calling_code(
                country_calling_code,
                PhoneNumberFormat::E164,
                &mut formatted_number,
            );
            return Ok(Cow::Owned(formatted_number));
        }
        // Note here that all NANPA formatting rules are contained by US, so we use
        // rules are contained by Russia. French Indian Ocean country rules are
        // contained by Réunion.
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let metadata =
            self.get_metadata_for_region_or_calling_code(country_calling_code, &region_code);

        if let Some(metadata) = metadata {
            if let Cow::Owned(s) = self.format_nsn(&formatted_number, metadata, number_format)? {
                formatted_number = s;
            }
            if let Some(formatted_extension) =
                Self::get_formatted_extension(phone_number, metadata, number_format)
            {
                formatted_number.push_str(&formatted_extension);
            }
            prefix_number_with_country_calling_code(
                country_calling_code,
                number_format,
                &mut formatted_number,
            );
        }
        Ok(Cow::Owned(formatted_number))
    }

    pub fn get_national_significant_number(&self, phone_number: &PhoneNumber) -> String {
        let zeros_start = if phone_number.italian_leading_zero() {
            let zero_count = usize::try_from(phone_number.number_of_leading_zeros()).unwrap_or(0);
            "0".repeat(zero_count)
        } else {
            "".to_string()
        };

        let mut buf = itoa::Buffer::new();
        let national_number = buf.format(phone_number.national_number());

        // If leading zero(s) have been set, we prefix this now. Note this is not a
        // national prefix. Ensure the number of leading zeros is at least 0 so we
        // don't crash in the case of malicious input.
        return fast_cat::concat_str!(&zeros_start, national_number);
    }

    /// Returns the region code that matches the specific country calling code. In
    /// the case of no region code being found, the unknown region code will be
    /// returned.
    pub fn get_region_code_for_country_code(&self, country_calling_code: i32) -> &str {
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        return region_codes
            .and_then(|mut codes| codes.next())
            .map(|v| v)
            .unwrap_or(RegionCode::get_unknown());
    }

    /// Returns the region codes that matches the specific country calling code. In
    /// the case of no region code being found, region_codes will be left empty.
    pub fn get_region_codes_for_country_calling_code(
        &self,
        country_calling_code: i32,
    ) -> Option<impl Iterator<Item = &str>> {
        // Create a IntRegionsPair with the country_code passed in, and use it to
        // locate the pair with the same country_code in the sorted vector.
        self.country_calling_code_to_region_code_map
            .binary_search_by_key(&country_calling_code, |(code, _)| *code)
            .ok()
            .map(|index| {
                self.country_calling_code_to_region_code_map[index]
                    .1
                    .iter()
                    .map(|v| v.as_str())
            })
    }

    pub fn get_metadata_for_region_or_calling_code(
        &self,
        country_calling_code: i32,
        region_code: &str,
    ) -> Option<&PhoneMetadata> {
        return if REGION_CODE_FOR_NON_GEO_ENTITY == region_code {
            self.country_code_to_non_geographical_metadata_map
                .get(&country_calling_code)
        } else {
            self.region_to_metadata_map.get(region_code)
        };
    }

    pub fn format_nsn<'b>(
        &self,
        phone_number: &'b str,
        metadata: &PhoneMetadata,
        number_format: PhoneNumberFormat,
    ) -> RegexResult<Cow<'b, str>> {
        self.format_nsn_with_carrier(phone_number, metadata, number_format, "")
    }

    fn format_nsn_with_carrier<'b>(
        &self,
        number: &'b str,
        metadata: &PhoneMetadata,
        number_format: PhoneNumberFormat,
        carrier_code: &str,
    ) -> RegexResult<Cow<'b, str>> {
        // When the intl_number_formats exists, we use that to format national number
        // for the INTERNATIONAL format instead of using the number_formats.
        let available_formats = if metadata.intl_number_format.len() == 0
            || number_format == PhoneNumberFormat::National
        {
            &metadata.number_format
        } else {
            &metadata.intl_number_format
        };
        let formatting_pattern =
            self.choose_formatting_pattern_for_number(available_formats, number)?;
        if let Some(formatting_pattern) = formatting_pattern {
            self.format_nsn_using_pattern_with_carrier(
                number,
                formatting_pattern,
                number_format,
                carrier_code,
            )
        } else {
            Ok(Cow::Borrowed(number))
        }
    }

    pub fn choose_formatting_pattern_for_number<'b>(
        &self,
        available_formats: &'b [NumberFormat],
        national_number: &str,
    ) -> RegexResult<Option<&'b NumberFormat>> {
        for format in available_formats {
            if !format
                .leading_digits_pattern
                // We always use the last leading_digits_pattern, as it is the most
                // detailed.
                .last()
                .map(|last| {
                    self.reg_exps
                        .regexp_cache
                        .get_regex(&last)
                        .and_then(|regex| Ok(regex.matches_start(national_number)))
                })
                // default not continue
                .unwrap_or(Ok(true))?
            {
                continue;
            }
            let pattern_to_match = self.reg_exps.regexp_cache.get_regex(format.pattern())?;
            if pattern_to_match.full_match(national_number) {
                return Ok(Some(format));
            }
        }
        return Ok(None);
    }

    // Note that carrier_code is optional - if an empty string, no carrier code
    // replacement will take place.
    fn format_nsn_using_pattern_with_carrier<'b>(
        &self,
        national_number: &'b str,
        formatting_pattern: &NumberFormat,
        number_format: PhoneNumberFormat,
        carrier_code: &str,
    ) -> RegexResult<Cow<'b, str>> {
        let mut number_format_rule = Cow::Borrowed(formatting_pattern.format());
        if matches!(number_format, PhoneNumberFormat::National)
            && carrier_code.len() > 0
            && formatting_pattern
                .domestic_carrier_code_formatting_rule()
                .len()
                > 0
        {
            // Replace the $CC in the formatting rule with the desired carrier code.
            let mut carrier_code_formatting_rule =
                Cow::Borrowed(formatting_pattern.domestic_carrier_code_formatting_rule());

            if let Cow::Owned(s) = self
                .reg_exps
                .carrier_code_pattern
                .replace(&carrier_code_formatting_rule, carrier_code)
            {
                carrier_code_formatting_rule = Cow::Owned(s);
            }
            if let Cow::Owned(s) = self
                .reg_exps
                .first_group_capturing_pattern
                .replace(&number_format_rule, carrier_code_formatting_rule)
            {
                number_format_rule = Cow::Owned(s);
            }
        } else {
            // Use the national prefix formatting rule instead.
            let national_prefix_formatting_rule =
                formatting_pattern.national_prefix_formatting_rule();

            if matches!(number_format, PhoneNumberFormat::National)
                && national_prefix_formatting_rule.len() > 0
            {
                // Apply the national_prefix_formatting_rule as the formatting_pattern
                // contains only information on how the national significant number
                // should be formatted at this point.
                if let Cow::Owned(s) = self
                    .reg_exps
                    .first_group_capturing_pattern
                    .replace(&number_format_rule, national_prefix_formatting_rule)
                {
                    number_format_rule = Cow::Owned(s);
                }
            }
        }

        let pattern_to_match = self
            .reg_exps
            .regexp_cache
            .get_regex(formatting_pattern.pattern())?;

        let mut formatted_number =
            pattern_to_match.replace_all(national_number, number_format_rule);

        if matches!(number_format, PhoneNumberFormat::RFC3966) {
            // First consume any leading punctuation, if any was present.
            if let Some(matches) = self
                .reg_exps
                .separator_pattern
                .find_start(&formatted_number)
            {
                let rest = &formatted_number[matches.end()..];
                formatted_number = Cow::Owned(rest.to_string());
            }
            // Then replace all separators with a "-".
            if let Cow::Owned(s) = self
                .reg_exps
                .separator_pattern
                .replace_all(&formatted_number, "-")
            {
                formatted_number = Cow::Owned(s)
            }
        }
        Ok(formatted_number)
    }

    /// Simple wrapper of FormatNsnUsingPatternWithCarrier for the common case of
    /// no carrier code.
    pub fn format_nsn_using_pattern<'b>(
        &self,
        national_number: &'b str,
        formatting_pattern: &NumberFormat,
        number_format: PhoneNumberFormat,
    ) -> RegexResult<Cow<'b, str>> {
        self.format_nsn_using_pattern_with_carrier(
            national_number,
            formatting_pattern,
            number_format,
            "",
        )
    }

    /// Returns the formatted extension of a phone number, if the phone number had an
    /// extension specified else None.
    fn get_formatted_extension(
        phone_number: &PhoneNumber,
        metadata: &PhoneMetadata,
        number_format: PhoneNumberFormat,
    ) -> Option<String> {
        if !phone_number.has_extension() || phone_number.extension().is_empty() {
            return None;
        }

        let prefix = if matches!(number_format, PhoneNumberFormat::RFC3966) {
            RFC3966_EXTN_PREFIX
        } else if metadata.has_preferred_extn_prefix() {
            metadata.preferred_extn_prefix()
        } else {
            DEFAULT_EXTN_PREFIX
        };
        Some(fast_cat::concat_str!(prefix, phone_number.extension()))
    }

    pub fn format_by_pattern(
        &self,
        phone_number: &PhoneNumber,
        number_format: PhoneNumberFormat,
        user_defined_formats: &[NumberFormat],
    ) -> RegexResult<String> {
        let country_calling_code = phone_number.country_code();
        // Note GetRegionCodeForCountryCode() is used because formatting information
        // contained in the metadata for US.
        let national_significant_number = self.get_national_significant_number(phone_number);
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let Some(metadata) =
            self.get_metadata_for_region_or_calling_code(country_calling_code, &region_code)
        else {
            return Ok(national_significant_number);
        };

        let formatting_pattern = self.choose_formatting_pattern_for_number(
            user_defined_formats,
            &national_significant_number,
        )?;

        let mut formatted_number = if let Some(formatting_pattern) = formatting_pattern {
            // Before we do a replacement of the national prefix pattern $NP with the
            // national prefix, we need to copy the rule so that subsequent replacements
            // for different numbers have the appropriate national prefix.
            let mut num_format_copy = formatting_pattern.clone();

            let national_prefix_formatting_rule =
                formatting_pattern.national_prefix_formatting_rule();
            if !national_prefix_formatting_rule.is_empty() {
                let national_prefix = metadata.national_prefix();
                if !national_prefix.is_empty() {
                    // Replace $NP with national prefix and $FG with the first group ($1).
                    let rule = national_prefix_formatting_rule
                        .replace("$NP", national_prefix)
                        .replace("$FG", "$1");
                    num_format_copy.set_national_prefix_formatting_rule(rule);
                } else {
                    // We don't want to have a rule for how to format the national prefix if
                    // there isn't one.
                    num_format_copy.clear_national_prefix_formatting_rule();
                }
            }
            self.format_nsn_using_pattern(
                &national_significant_number,
                &num_format_copy,
                number_format,
            )?
            .to_string()
        } else {
            national_significant_number
        };
        if let Some(extension) =
            Self::get_formatted_extension(phone_number, metadata, PhoneNumberFormat::National)
        {
            formatted_number.push_str(&extension);
        }
        prefix_number_with_country_calling_code(
            country_calling_code,
            number_format,
            &mut formatted_number,
        );
        Ok(formatted_number)
    }

    pub fn format_national_number_with_carrier_code(
        &self,
        phone_number: &PhoneNumber,
        carrier_code: &str,
    ) -> RegexResult<String> {
        let country_calling_code = phone_number.country_code();
        let national_significant_number = self.get_national_significant_number(phone_number);
        let region_code = self.get_region_code_for_country_code(country_calling_code);

        // Note GetRegionCodeForCountryCode() is used because formatting information
        // contained in the metadata for US.
        let Some(metadata) =
            self.get_metadata_for_region_or_calling_code(country_calling_code, &region_code)
        else {
            return Ok(national_significant_number);
        };

        let mut formatted_number = owned_from_cow_or!(
            self.format_nsn_with_carrier(
                &national_significant_number,
                metadata,
                PhoneNumberFormat::National,
                carrier_code,
            )?,
            national_significant_number
        );
        if let Some(formatted_extension) =
            Self::get_formatted_extension(phone_number, metadata, PhoneNumberFormat::National)
        {
            formatted_number.push_str(&formatted_extension);
        }

        prefix_number_with_country_calling_code(
            country_calling_code,
            PhoneNumberFormat::National,
            &mut formatted_number,
        );

        Ok(formatted_number)
    }

    pub fn format_national_number_with_preferred_carrier_code(
        &self,
        phone_number: &PhoneNumber,
        fallback_carrier_code: &str,
    ) -> RegexResult<String> {
        let carrier_code = if !phone_number.preferred_domestic_carrier_code().is_empty() {
            phone_number.preferred_domestic_carrier_code()
        } else {
            fallback_carrier_code
        };
        self.format_national_number_with_carrier_code(phone_number, carrier_code)
    }

    fn has_valid_country_calling_code(&self, country_calling_code: i32) -> bool {
        // Create an IntRegionsPair with the country_code passed in, and use it to
        // locate the pair with the same country_code in the sorted vector.

        return self
            .country_calling_code_to_region_code_map
            .binary_search_by_key(&country_calling_code, |(k, _)| *k)
            .is_ok();
    }

    pub fn format_number_for_mobile_dialing<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        calling_from: &str,
        with_formatting: bool,
    ) -> InternalLogicResult<Cow<'b, str>> {
        let country_calling_code = phone_number.country_code();
        if !self.has_valid_country_calling_code(country_calling_code) {
            return if phone_number.has_raw_input() {
                Ok(Cow::Borrowed(phone_number.raw_input()))
            } else {
                Ok(Cow::Borrowed(""))
            };
        }

        let mut formatted_number = String::new();
        // Clear the extension, as that part cannot normally be dialed together with
        // the main number.
        let mut number_no_extension = phone_number.clone();
        number_no_extension.clear_extension();
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let number_type = self.get_number_type(&number_no_extension)?;
        let is_valid_number = !matches!(number_type, PhoneNumberType::Unknown);
        if calling_from == region_code {
            let is_fixed_line_or_mobile = matches!(
                number_type,
                PhoneNumberType::FixedLine
                    | PhoneNumberType::FixedLineOrMobile
                    | PhoneNumberType::Mobile
            );
            // Carrier codes may be needed in some countries. We handle this here.
            if (region_code == "BR") && (is_fixed_line_or_mobile) {
                // Historically, we set this to an empty string when parsing with raw
                // input if none was found in the input string. However, this doesn't
                // result in a number we can dial. For this reason, we treat the empty
                // string the same as if it isn't set at all.
                if !number_no_extension
                    .preferred_domestic_carrier_code()
                    .is_empty()
                {
                    formatted_number = self.format_national_number_with_preferred_carrier_code(
                        &number_no_extension,
                        "",
                    )?;
                } else {
                    // Brazilian fixed line and mobile numbers need to be dialed with a
                    // carrier code when called within Brazil. Without that, most of the
                    // carriers won't connect the call. Because of that, we return an empty
                    // string here.
                    // IDK BUT KEPPET
                    formatted_number.clear();
                }
            } else if country_calling_code == NANPA_COUNTRY_CODE {
                // For NANPA countries, we output international format for numbers that
                // can be dialed internationally, since that always works, except for
                // numbers which might potentially be short numbers, which are always
                // dialled in national format.
                let region_metadata = self
                    .region_to_metadata_map
                    .get(calling_from)
                    .ok_or(InvalidMetadataForValidRegionError {})?;
                let national_number = self.get_national_significant_number(&number_no_extension);
                let format = if self.can_be_internationally_dialled(&number_no_extension)?
                    && !test_number_length_with_unknown_type(&national_number, region_metadata)
                        .is_err_and(|e| matches!(e, ValidationError::TooShort))
                {
                    PhoneNumberFormat::International
                } else {
                    PhoneNumberFormat::National
                };
                if let Cow::Owned(s) = self.format(&number_no_extension, format)? {
                    formatted_number = s;
                }
            } else {
                // For non-geographical countries, and Mexican, Chilean and Uzbek fixed
                // line and mobile numbers, we output international format for numbers
                // that can be dialed internationally as that always works.
                let format = if (region_code == REGION_CODE_FOR_NON_GEO_ENTITY ||
                        // MX fixed line and mobile numbers should always be formatted in
                        // international format, even when dialed within MX. For national
                        // format to work, a carrier code needs to be used, and the correct
                        // carrier code depends on if the caller and callee are from the same
                        // local area. It is trickier to get that to work correctly than
                        // using international format, which is tested to work fine on all
                        // carriers.
                        // CL fixed line numbers need the national prefix when dialing in the
                        // national format, but don't have it when used for display. The
                        // reverse is true for mobile numbers. As a result, we output them in
                        // the international format to make it work.
                        // UZ mobile and fixed-line numbers have to be formatted in
                        // international format or prefixed with special codes like 03, 04
                        // (for fixed-line) and 05 (for mobile) for dialling successfully
                        // from mobile devices. As we do not have complete information on
                        // special codes and to be consistent with formatting across all
                        // phone types we return the number in international format here.
                        ((region_code == "MX" ||
                        region_code == "CL" ||
                        region_code == "UZ") &&
                        is_fixed_line_or_mobile))
                    && self.can_be_internationally_dialled(&number_no_extension)?
                {
                    PhoneNumberFormat::International
                } else {
                    PhoneNumberFormat::National
                };
                if let Cow::Owned(s) = self.format(&number_no_extension, format)? {
                    formatted_number = s;
                }
            }
        } else if is_valid_number && self.can_be_internationally_dialled(&number_no_extension)? {
            // We assume that short numbers are not diallable from outside their
            // region, so if a number is not a valid regular length phone number, we
            // treat it as if it cannot be internationally dialled.
            let format = if with_formatting {
                PhoneNumberFormat::International
            } else {
                PhoneNumberFormat::E164
            };
            return Ok(Cow::Owned(owned_from_cow_or!(
                self.format(&number_no_extension, format)?,
                formatted_number
            )));
        }
        if !with_formatting {
            Ok(Cow::Owned(
                self.normalize_diallable_chars_only(&formatted_number),
            ))
        } else {
            Ok(Cow::Owned(formatted_number))
        }
    }

    pub fn get_number_type(
        &self,
        phone_number: &PhoneNumber,
    ) -> InternalLogicResult<PhoneNumberType> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) =
            self.get_metadata_for_region_or_calling_code(phone_number.country_code(), region_code)
        else {
            return Ok(PhoneNumberType::Unknown);
        };
        let national_significant_number = self.get_national_significant_number(phone_number);
        Ok(self.get_number_type_helper(&national_significant_number, metadata))
    }

    pub fn get_region_code_for_number(
        &self,
        phone_number: &PhoneNumber,
    ) -> InternalLogicResult<&str> {
        let country_calling_code = phone_number.country_code();
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        let Some(region_codes) = region_codes
            .map(|codes| codes.collect::<Vec<_>>())
            .filter(|codes| codes.len() > 0)
        else {
            trace!(
                "Missing/invalid country calling code ({})",
                country_calling_code
            );
            return Ok(RegionCode::get_unknown());
        };
        if region_codes.len() == 1 {
            return Ok(region_codes[0]);
        } else {
            self.get_region_code_for_number_from_region_list(phone_number, &region_codes)
        }
    }

    pub fn get_region_code_for_number_from_region_list<'b>(
        &self,
        phone_number: &PhoneNumber,
        region_codes: &[&'b str],
    ) -> InternalLogicResult<&'b str> {
        let national_number = self.get_national_significant_number(phone_number);
        for code in region_codes {
            // Metadata cannot be NULL because the region codes come from the country
            // calling code map.
            let metadata = &self
                .region_to_metadata_map
                .get(*code)
                .ok_or(InvalidMetadataForValidRegionError {})?;
            if metadata.has_leading_digits() {
                if self
                    .reg_exps
                    .regexp_cache
                    .get_regex(metadata.leading_digits())?
                    .matches_start(&national_number)
                {
                    return Ok(code);
                }
            } else if self.get_number_type_helper(&national_number, metadata)
                != PhoneNumberType::Unknown
            {
                return Ok(code);
            }
        }
        return Ok(RegionCode::get_unknown());
    }

    fn get_number_type_helper(
        &self,
        national_number: &str,
        metadata: &PhoneMetadata,
    ) -> PhoneNumberType {
        if !self.is_number_matching_desc(national_number, &metadata.general_desc) {
            trace!(
                "Number '{national_number}' type unknown - doesn't match general national number pattern"
            );
            return PhoneNumberType::Unknown;
        }
        if self.is_number_matching_desc(national_number, &metadata.premium_rate) {
            trace!("Number '{national_number}' is a premium number.");
            return PhoneNumberType::PremiumRate;
        }
        if self.is_number_matching_desc(national_number, &metadata.toll_free) {
            trace!("Number '{national_number}' is a toll-free number.");
            return PhoneNumberType::TollFree;
        }
        if self.is_number_matching_desc(national_number, &metadata.shared_cost) {
            trace!("Number '{national_number}' is a shared cost number.");
            return PhoneNumberType::SharedCost;
        }
        if self.is_number_matching_desc(national_number, &metadata.voip) {
            trace!("Number '{national_number}' is a VOIP (Voice over IP) number.");
            return PhoneNumberType::VoIP;
        }
        if self.is_number_matching_desc(national_number, &metadata.personal_number) {
            trace!("Number '{national_number}' is a personal number.");
            return PhoneNumberType::PersonalNumber;
        }
        if self.is_number_matching_desc(national_number, &metadata.pager) {
            trace!("Number '{national_number}' is a pager number.");
            return PhoneNumberType::Pager;
        }
        if self.is_number_matching_desc(national_number, &metadata.uan) {
            trace!("Number '{national_number}' is a UAN.");
            return PhoneNumberType::UAN;
        }
        if self.is_number_matching_desc(national_number, &metadata.voicemail) {
            trace!("Number '{national_number}' is a voicemail number.");
            return PhoneNumberType::VoiceMail;
        }

        let is_fixed_line = self.is_number_matching_desc(national_number, &metadata.fixed_line);
        if is_fixed_line {
            if metadata.same_mobile_and_fixed_line_pattern() {
                trace!(
                    "Number '{national_number}': fixed-line and mobile patterns equal,\
                 number is fixed-line or mobile"
                );
                return PhoneNumberType::FixedLineOrMobile;
            } else if self.is_number_matching_desc(national_number, &metadata.mobile) {
                trace!(
                    "Number '{national_number}': Fixed-line and mobile patterns differ, but number is \
                        still fixed-line or mobile"
                );
                return PhoneNumberType::FixedLineOrMobile;
            }
            trace!("Number '{national_number}' is a fixed line number.");
            return PhoneNumberType::FixedLine;
        }
        // Otherwise, test to see if the number is mobile. Only do this if certain
        // that the patterns for mobile and fixed line aren't the same.
        if !metadata.same_mobile_and_fixed_line_pattern()
            && self.is_number_matching_desc(national_number, &metadata.mobile)
        {
            trace!("Number '{national_number}' is a mobile number.");
            return PhoneNumberType::Mobile;
        }
        trace!(
            "Number'{national_number}' type unknown - doesn\'t match any specific number type pattern."
        );
        return PhoneNumberType::Unknown;
    }

    pub fn is_number_matching_desc(
        &self,
        national_number: &str,
        number_desc: &PhoneNumberDesc,
    ) -> bool {
        // Check if any possible number lengths are present; if so, we use them to
        // avoid checking the validation pattern if they don't match. If they are
        // absent, this means they match the general description, which we have
        // already checked before checking a specific number type.
        let actual_length = national_number.len() as i32;
        if number_desc.possible_length.len() > 0
            && !number_desc.possible_length.contains(&actual_length)
        {
            return false;
        }
        // very common name, so specify mod
        helper_functions::is_match(&self.matcher_api, national_number, number_desc)
    }

    pub fn can_be_internationally_dialled(
        &self,
        phone_number: &PhoneNumber,
    ) -> InternalLogicResult<bool> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) = self.region_to_metadata_map.get(region_code) else {
            // Note numbers belonging to non-geographical entities (e.g. +800 numbers)
            // are always internationally diallable, and will be caught here.
            return Ok(true);
        };
        let national_significant_number = self.get_national_significant_number(phone_number);
        return Ok(!self.is_number_matching_desc(
            &national_significant_number,
            &metadata.no_international_dialling,
        ));
    }

    pub fn normalize_diallable_chars_only(&self, phone_number: &str) -> String {
        normalize_helper(&self.reg_exps.diallable_char_mappings, true, phone_number)
    }

    pub fn normalize_digits_only<'a>(&self, phone_number: &'a str) -> String {
        dec_from_char::normalize_decimals_filtering(phone_number)
    }

    pub fn format_out_of_country_calling_number<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        calling_from: &str,
    ) -> InternalLogicResult<Cow<'a, str>> {
        let Some(metadata_calling_from) = self.region_to_metadata_map.get(calling_from) else {
            trace!(
                "Trying to format number from invalid region {calling_from}\
              . International formatting applied."
            );
            return Ok(self.format(phone_number, PhoneNumberFormat::International)?);
        };
        let country_code = phone_number.country_code();
        let national_significant_number = self.get_national_significant_number(phone_number);
        if !self.has_valid_country_calling_code(country_code) {
            return Ok(Cow::Owned(national_significant_number));
        }
        if country_code == NANPA_COUNTRY_CODE {
            if self.nanpa_regions.contains(calling_from) {
                let mut buf = itoa::Buffer::new();
                // prefix it with the country calling code.
                return Ok(Cow::Owned(fast_cat::concat_str!(
                    buf.format(country_code),
                    " ",
                    &self.format(phone_number, PhoneNumberFormat::National)?,
                )));
            }
        } else if country_code == metadata_calling_from.country_code() {
            // If neither region is a NANPA region, then we check to see if the
            // country calling code of the number and the country calling code of the
            // region we are calling from are the same.
            // need not be dialled. This also applies when dialling within a region, so
            // this if clause covers both these cases.
            // Technically this is the case for dialling from la Réunion to other
            // overseas departments of France (French Guiana, Martinique, Guadeloupe),
            // but not vice versa - so we don't cover this edge case for now and for
            // those cases return the version including country calling code.
            // Details here:
            // http://www.petitfute.com/voyage/225-info-pratiques-reunion
            return Ok(self.format(phone_number, PhoneNumberFormat::National)?);
        }
        // Metadata cannot be NULL because we checked 'IsValidRegionCode()' above.
        let international_prefix = metadata_calling_from.international_prefix();

        // In general, if there is a preferred international prefix, use that.
        // international format of the number is returned since we would not know
        // which one to use.
        let international_prefix_for_formatting =
            if metadata_calling_from.has_preferred_international_prefix() {
                metadata_calling_from.preferred_international_prefix()
            } else if self
                .reg_exps
                .single_international_prefix
                .full_match(international_prefix)
            {
                international_prefix
            } else {
                ""
            };

        let region_code = self.get_region_code_for_country_code(country_code);
        // Metadata cannot be NULL because the country_code is valid.
        let metadata_for_region = self
            .get_metadata_for_region_or_calling_code(country_code, region_code)
            .ok_or(InvalidMetadataForValidRegionError {})?;

        let formatted_nsn = self.format_nsn(
            &national_significant_number,
            metadata_for_region,
            PhoneNumberFormat::International,
        )?;

        let mut formatted_number = owned_from_cow_or!(formatted_nsn, national_significant_number);

        if let Some(extension) = Self::get_formatted_extension(
            phone_number,
            metadata_for_region,
            PhoneNumberFormat::International,
        ) {
            formatted_number.push_str(&extension);
        }

        return Ok(Cow::Owned(
            if !international_prefix_for_formatting.is_empty() {
                let mut buf = itoa::Buffer::new();
                fast_cat::concat_str!(
                    international_prefix_for_formatting,
                    " ",
                    buf.format(country_code),
                    " ",
                    &formatted_number,
                )
            } else {
                prefix_number_with_country_calling_code(
                    country_code,
                    PhoneNumberFormat::International,
                    &mut formatted_number,
                );
                formatted_number
            },
        ));
    }

    fn has_formatting_pattern_for_number(&self, phone_number: &PhoneNumber) -> RegexResult<bool> {
        let country_calling_code = phone_number.country_code();
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let Some(metadata) =
            self.get_metadata_for_region_or_calling_code(country_calling_code, region_code)
        else {
            return Ok(false);
        };
        let national_number = self.get_national_significant_number(phone_number);
        let format_rule =
            self.choose_formatting_pattern_for_number(&metadata.number_format, &national_number);
        return format_rule.map(|rule| rule.is_some());
    }

    pub fn format_in_original_format<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        region_calling_from: &str,
    ) -> InternalLogicResult<Cow<'a, str>> {
        if phone_number.has_raw_input() && !self.has_formatting_pattern_for_number(phone_number)? {
            // We check if we have the formatting pattern because without that, we might
            // format the number as a group without national prefix.
            return Ok(Cow::Borrowed(phone_number.raw_input()));
        }
        if !phone_number.has_country_code_source() {
            return Ok(self.format(phone_number, PhoneNumberFormat::National)?);
        }
        let formatted_number = match phone_number.country_code_source() {
            CountryCodeSource::FROM_NUMBER_WITH_PLUS_SIGN => {
                self.format(phone_number, PhoneNumberFormat::International)?
            }
            CountryCodeSource::FROM_NUMBER_WITH_IDD => {
                self.format_out_of_country_calling_number(phone_number, region_calling_from)?
            }
            CountryCodeSource::FROM_NUMBER_WITHOUT_PLUS_SIGN => Cow::Owned(
                self.format(phone_number, PhoneNumberFormat::International)?[1..].to_string(),
            ),
            CountryCodeSource::FROM_DEFAULT_COUNTRY
            | CountryCodeSource::UNSPECIFIED => 'default_block: {
                let format_national = || self.format(phone_number, PhoneNumberFormat::National);

                let region_code =
                    self.get_region_code_for_country_code(phone_number.country_code());
                // We strip non-digits from the NDD here, and from the raw input later, so
                // that we can compare them easily.
                let Some(national_prefix) =
                    self.get_ndd_prefix_for_region(region_code, true /* strip non-digits */) else {
                    break 'default_block format_national()?;
                };
                let Some(metadata) = self.region_to_metadata_map.get(region_code) else {
                    // If the region doesn't have a national prefix at all, we can safely
                    // return the national format without worrying about a national prefix
                    // being added.
                    break 'default_block format_national()?;
                };
                // Otherwise, we check if the original number was entered with a national
                // prefix.
                if self.raw_input_contains_national_prefix(
                    phone_number.raw_input(),
                    &national_prefix,
                    region_code,
                )? {
                    // If so, we can safely return the national format.
                    break 'default_block format_national()?;
                }
                // Metadata cannot be NULL here because GetNddPrefixForRegion() (above)
                // leaves the prefix empty if there is no metadata for the region.
                let national_number = self.get_national_significant_number(phone_number);
                // This shouldn't be NULL, because we have checked that above with
                // HasFormattingPatternForNumber.
                let format_rule = self.choose_formatting_pattern_for_number(
                    &metadata.number_format,
                    &national_number,
                )?;
                // The format rule could still be NULL here if the national number was 0
                // and there was no raw input (this should not be possible for numbers
                // generated by the phonenumber library as they would also not have a
                // country calling code and we would have exited earlier).
                let Some(format_rule) = format_rule else {
                    break 'default_block format_national()?;
                };
                // When the format we apply to this number doesn't contain national
                // prefix, we can just return the national format.
                // TODO: Refactor the code below with the code in
                // IsNationalPrefixPresentIfRequired.
                let candidate_national_prefix_rule = format_rule.national_prefix_formatting_rule();
                // We assume that the first-group symbol will never be _before_ the
                // national prefix.
                let candidate_national_prefix_rule_empty =
                    if !candidate_national_prefix_rule.is_empty() {
                        let Some(index_of_first_group) = candidate_national_prefix_rule.find("$1")
                        else {
                            error!(
                                "First group missing in national prefix rule: {}",
                                candidate_national_prefix_rule
                            );
                            break 'default_block format_national()?;
                        };
                        let candidate_national_prefix_rule =
                            &candidate_national_prefix_rule[..index_of_first_group];
                        self.normalize_digits_only(&candidate_national_prefix_rule)
                            .is_empty()
                    } else {
                        true
                    };
                if candidate_national_prefix_rule_empty {
                    // National prefix not used when formatting this number.
                    break 'default_block format_national()?;
                };
                // Otherwise, we need to remove the national prefix from our output.
                let mut number_format = format_rule.clone();
                number_format.clear_national_prefix_formatting_rule();
                Cow::Owned(self.format_by_pattern(
                    phone_number,
                    PhoneNumberFormat::National,
                    &[number_format],
                )?)
            }
        };
        // If no digit is inserted/removed/modified as a result of our formatting, we
        // return the formatted phone number; otherwise we return the raw input the
        // user entered.
        if !formatted_number.is_empty() && !phone_number.raw_input().is_empty() {
            let normalized_formatted_number =
                self.normalize_diallable_chars_only(&formatted_number);
            let normalized_raw_input =
                self.normalize_diallable_chars_only(phone_number.raw_input());
            if normalized_formatted_number != normalized_raw_input {
                return Ok(Cow::Borrowed(phone_number.raw_input()));
            }
        }
        Ok(formatted_number)
    }

    /// Check if raw_input, which is assumed to be in the national format, has a
    /// national prefix. The national prefix is assumed to be in digits-only form.
    fn raw_input_contains_national_prefix(
        &self,
        raw_input: &str,
        national_prefix: &str,
        region_code: &str,
    ) -> InternalLogicResult<bool> {
        let normalized_national_number = self.normalize_digits_only(raw_input);
        if normalized_national_number.starts_with(national_prefix) {
            // Some Japanese numbers (e.g. 00777123) might be mistaken to contain
            // the national prefix when written without it (e.g. 0777123) if we just
            // do prefix matching. To tackle that, we check the validity of the
            // number if the assumed national prefix is removed (777123 won't be
            // valid in Japan).
            if let Ok(number_without_national_prefix) = self.parse(
                &normalized_national_number[national_prefix.len()..],
                region_code,
            ) {
                return self.is_valid_number(&number_without_national_prefix);
            }
        }
        Ok(false)
    }

    pub fn parse(&self, number_to_parse: &str, default_region: &str) -> ParseResult<PhoneNumber> {
        self.parse_helper(number_to_parse, default_region, false, true)
    }

    pub fn parse_and_keep_raw_input(
        &self,
        number_to_parse: &str,
        default_region: &str,
    ) -> ParseResult<PhoneNumber> {
        self.parse_helper(number_to_parse, default_region, true, true)
    }

    pub fn is_valid_number(&self, phone_number: &PhoneNumber) -> InternalLogicResult<bool> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        return Ok(self.is_valid_number_for_region(phone_number, region_code));
    }

    pub fn is_valid_number_for_region(
        &self,
        phone_number: &PhoneNumber,
        region_code: &str,
    ) -> bool {
        let country_code = phone_number.country_code();
        let metadata = self.get_metadata_for_region_or_calling_code(country_code, region_code);
        if let Some(metadata) = metadata.filter(|metadata| {
            !(REGION_CODE_FOR_NON_GEO_ENTITY != region_code
                && country_code != metadata.country_code())
        }) {
            let national_number = self.get_national_significant_number(phone_number);
            !matches!(
                self.get_number_type_helper(&national_number, metadata),
                PhoneNumberType::Unknown
            )
        } else {
            false
        }
    }

    pub fn format_out_of_country_keeping_alpha_chars<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        calling_from: &str,
    ) -> InternalLogicResult<Cow<'a, str>> {
        // If there is no raw input, then we can't keep alpha characters because there
        // aren't any. In this case, we return FormatOutOfCountryCallingNumber.
        if phone_number.raw_input().is_empty() {
            return self.format_out_of_country_calling_number(phone_number, calling_from);
        }

        let country_code = phone_number.country_code();
        if !self.has_valid_country_calling_code(country_code) {
            return Ok(phone_number.raw_input().into());
        }
        // Strip any prefix such as country calling code, IDD, that was present. We do
        // this by comparing the number in raw_input with the parsed number.
        // Normalize punctuation. We retain number grouping symbols such as " " only.
        let mut normalized_raw_input = helper_functions::normalize_helper(
            &self.reg_exps.all_plus_number_grouping_symbols,
            true,
            phone_number.raw_input(),
        );
        // Now we trim everything before the first three digits in the parsed number.
        // We choose three because all valid alpha numbers have 3 digits at the start
        // - if it does not, then we don't trim anything at all. Similarly, if the
        // national number was less than three digits, we don't trim anything at all.
        let national_number = self.get_national_significant_number(phone_number);
        if national_number.len() > 3 {
            let first_national_number_digit = normalized_raw_input.find(&national_number[0..3]);
            if let Some(first_national_number_digit) = first_national_number_digit {
                normalized_raw_input.drain(0..first_national_number_digit);
            }
        }
        let metadata = self.region_to_metadata_map.get(calling_from);
        if country_code == NANPA_COUNTRY_CODE {
            if self.nanpa_regions.contains(calling_from) {
                let mut buf = itoa::Buffer::new();

                return Ok(fast_cat::concat_str!(
                    buf.format(country_code),
                    " ",
                    &normalized_raw_input
                )
                .into());
            }
        } else if let Some(metadata) =
            metadata.filter(|metadata| country_code == metadata.country_code())
        {
            let Some(formatting_pattern) = self
                .choose_formatting_pattern_for_number(&metadata.number_format, &national_number)?
            else {
                // If no pattern above is matched, we format the original input.
                return Ok(normalized_raw_input.into());
            };
            let mut new_format = formatting_pattern.clone();
            // The first group is the first group of digits that the user wrote
            // together.
            new_format.set_pattern("(\\d+)(.*)".to_owned());
            // Here we just concatenate them back together after the national prefix
            // has been fixed.
            new_format.set_format("$1$2".to_owned());
            // Now we format using this pattern instead of the default pattern, but
            // with the national prefix prefixed if necessary.
            // This will not work in the cases where the pattern (and not the
            // leading digits) decide whether a national prefix needs to be used, since
            // we have overridden the pattern to match anything, but that is not the
            // case in the metadata to date.
            return Ok(self
                .format_nsn_using_pattern(
                    &normalized_raw_input,
                    &new_format,
                    PhoneNumberFormat::National,
                )
                .map(|cow| Cow::Owned(cow.into_owned()))?);
        }

        // If an unsupported region-calling-from is entered, or a country with
        // multiple international prefixes, the international format of the number is
        // returned, unless there is a preferred international prefix.
        let international_prefix_for_formatting = metadata.map(|metadata| {
            let international_prefix = metadata.international_prefix();
            if self
                .reg_exps
                .single_international_prefix
                .full_match(international_prefix)
            {
                international_prefix
            } else {
                metadata.preferred_international_prefix()
            }
        });
        let formatted_number = if let Some(international_prefix_for_formatting) =
            international_prefix_for_formatting
        {
            let mut buf = itoa::Buffer::new();
            fast_cat::concat_str!(
                international_prefix_for_formatting,
                " ",
                buf.format(country_code),
                " ",
                &normalized_raw_input
            )
        } else {
            // Invalid region entered as country-calling-from (so no metadata was found
            // for it) or the region chosen has multiple international dialling
            // prefixes.
            if !self.region_to_metadata_map.contains_key(calling_from) {
                trace!(
                    "Trying to format number from invalid region {}. International formatting applied.",
                    calling_from
                );
            }
            let mut formatted_number = normalized_raw_input;
            prefix_number_with_country_calling_code(
                country_code,
                PhoneNumberFormat::International,
                &mut formatted_number,
            );
            formatted_number
        };
        let region_code = self.get_region_code_for_country_code(country_code);
        // Metadata cannot be null because the country code is valid.
        let metadata_for_region = self
            .get_metadata_for_region_or_calling_code(country_code, region_code)
            .ok_or(InvalidMetadataForValidRegionError {})?;

        // Strip any extension
        let (phone_number_without_extension, _) = self.maybe_strip_extension(&formatted_number);
        // Append the formatted extension
        let extension = Self::get_formatted_extension(
            phone_number,
            metadata_for_region,
            PhoneNumberFormat::International,
        );
        Ok(if let Some(extension) = extension {
            fast_cat::concat_str!(phone_number_without_extension, &extension)
        } else {
            phone_number_without_extension.to_string()
        }
        .into())
    }

    /// Returns whether the value of phoneContext follows the syntax defined in
    /// RFC3966.
    pub fn is_phone_context_valid(&self, phone_context: &str) -> bool {
        if phone_context.is_empty() {
            return false;
        }

        // Does phone-context value match pattern of global-number-digits or
        // domainname
        return self
            .reg_exps
            .rfc3966_global_number_digits_pattern
            .full_match(phone_context)
            || self
                .reg_exps
                .rfc3966_domainname_pattern
                .full_match(phone_context);
    }

    /// Converts number_to_parse to a form that we can parse and write it to
    /// national_number if it is written in RFC3966; otherwise extract a possible
    /// number out of it and write to national_number.
    pub fn build_national_number_for_parsing(&self, number_to_parse: &str) -> ParseResult<String> {
        let index_of_phone_context = number_to_parse.find(RFC3966_PHONE_CONTEXT);

        let mut national_number =
            String::with_capacity(number_to_parse.len() + RFC3966_PREFIX.len());

        // IMPORTANT RUST NOTE: in original c++ code function IsPhoneContextValid
        // always returns `true` if index of phone context is NULL (=> phone context is NULL)
        // if anything changes that logic MUST change.
        if let Some(index_of_phone_context) = index_of_phone_context {
            let phone_context =
                Self::extract_phone_context(number_to_parse, index_of_phone_context);
            if !self.is_phone_context_valid(phone_context) {
                trace!("The phone-context value for phone number {number_to_parse} is invalid.");
                return Err(NotANumberError::InvalidPhoneContext.into());
            }
            // If the phone context contains a phone number prefix, we need to capture
            // it, whereas domains will be ignored.
            if phone_context.starts_with(PLUS_SIGN) {
                // Additional parameters might follow the phone context. If so, we will
                // remove them here because the parameters after phone context are not
                // important for parsing the phone number.
                national_number.push_str(phone_context)
            };

            // Now append everything between the "tel:" prefix and the phone-context.
            // This should include the national number, an optional extension or
            // isdn-subaddress component. Note we also handle the case when "tel:" is
            // missing, as we have seen in some of the phone number inputs. In that
            // case, we append everything from the beginning.
            let index_of_rfc_prefix = number_to_parse.find(RFC3966_PREFIX);
            let index_of_national_number = index_of_rfc_prefix.map_or(0, |index_of_rfc_prefix| {
                index_of_rfc_prefix + RFC3966_PREFIX.len()
            });
            national_number
                .push_str(&number_to_parse[index_of_national_number..index_of_phone_context]);
        } else {
            // Extract a possible number from the string passed in (this strips leading
            // characters that could not be the start of a phone number.)
            national_number.push_str(self.extract_possible_number(number_to_parse)?);
        }

        // Delete the isdn-subaddress and everything after it if it is present. Note
        // extension won't appear at the same time with isdn-subaddress according to
        // paragraph 5.3 of the RFC3966 spec.
        let index_of_isdn = national_number.find(RFC3966_ISDN_SUBADDRESS);
        if let Some(index_of_isdn) = index_of_isdn {
            national_number.truncate(index_of_isdn);
        }
        // If both phone context and isdn-subaddress are absent but other parameters
        // are present, the parameters are left in nationalNumber. This is because
        // we are concerned about deleting content from a potential number string
        // when there is no strong evidence that the number is actually written in
        // RFC3966.
        return Ok(national_number);
    }

    /// Extracts the value of the phone-context parameter of number_to_extract_from
    /// where the index of ";phone-context=" is parameter index_of_phone_context,
    /// following the syntax defined in RFC3966.
    ///
    /// Returns the extracted `Some(possibly empty)`, or a `None` if no
    /// phone-context parameter is found.
    pub fn extract_phone_context<'a>(
        number_to_extract_from: &'a str,
        index_of_phone_context: usize,
    ) -> &'a str {
        let phone_context_start = index_of_phone_context + RFC3966_PHONE_CONTEXT.len();
        // If phone-context parameter is empty
        if phone_context_start >= number_to_extract_from.len() {
            return "";
        }

        let phone_context_end = number_to_extract_from[phone_context_start..].find(';');
        // If phone-context is not the last parameter

        if let Some(phone_context_end) = phone_context_end {
            &number_to_extract_from[phone_context_start..phone_context_end + phone_context_start]
        } else {
            &number_to_extract_from[phone_context_start..]
        }
    }

    /// Attempts to extract a possible number from the string passed in. This
    /// currently strips all leading characters that could not be used to start a
    /// phone number. Characters that can be used to start a phone number are
    /// defined in the valid_start_char_pattern. If none of these characters are
    /// found in the number passed in, an empty string is returned. This function
    /// also attempts to strip off any alternative extensions or endings if two or
    /// more are present, such as in the case of: (530) 583-6985 x302/x2303. The
    /// second extension here makes this actually two phone numbers, (530) 583-6985
    /// x302 and (530) 583-6985 x2303. We remove the second extension so that the
    /// first number is parsed correctly.
    pub fn extract_possible_number<'a>(
        &self,
        phone_number: &'a str,
    ) -> ExtractNumberResult<&'a str> {
        // Rust note: skip UTF-8 validation since in rust strings are already UTF-8 valid
        let mut i: usize = 0;
        for c in phone_number.chars() {
            if self
                .reg_exps
                .valid_start_char_pattern
                .full_match(&phone_number[i..i + c.len_utf8()])
            {
                break;
            }
            i += c.len_utf8();
        }

        if i == phone_number.len() {
            // No valid start character was found. extracted_number should be set to
            // empty string.
            return Err(ExtractNumberError::NoValidStartCharacter);
        }

        let mut extracted_number = &phone_number[i..];
        extracted_number = self.trim_unwanted_end_chars(extracted_number);
        if extracted_number.len() == 0 {
            return Err(ExtractNumberError::NotANumber);
        }

        // Now remove any extra numbers at the end.
        return Ok(self
            .reg_exps
            .capture_up_to_second_number_start_pattern
            .captures(&extracted_number)
            .and_then(| c | c.get(1))
            .map(move |m| m.as_str())
            .unwrap_or(extracted_number));
    }

    pub fn is_possible_number(&self, phone_number: &PhoneNumber) -> bool {
        self.is_possible_number_with_reason(phone_number).is_ok()
    }

    pub fn is_possible_number_for_type(
        &self,
        phone_number: &PhoneNumber,
        phone_number_type: PhoneNumberType,
    ) -> bool {
        self.is_possible_number_for_type_with_reason(phone_number, phone_number_type)
            .is_ok()
    }

    pub fn is_possible_number_for_string(
        &self,
        phone_number: &str,
        region_dialing_from: &str,
    ) -> bool {
        match self.parse(phone_number, region_dialing_from) {
            Ok(number_proto) => self.is_possible_number(&number_proto),

            Err(err) => {
                trace!(
                    "Error occurred while parsing given number: {}: {:?}",
                    phone_number, err
                );
                false
            }
        }
    }

    pub fn is_possible_number_with_reason(&self, phone_number: &PhoneNumber) -> ValidationResult {
        self.is_possible_number_for_type_with_reason(phone_number, PhoneNumberType::Unknown)
    }

    pub fn is_possible_number_for_type_with_reason(
        &self,
        phone_number: &PhoneNumber,
        phone_number_type: PhoneNumberType,
    ) -> ValidationResult {
        let national_number = self.get_national_significant_number(phone_number);
        let country_code = phone_number.country_code();
        // Note: For regions that share a country calling code, like NANPA numbers, we
        // just use the rules from the default region (US in this case) since the
        // GetRegionCodeForNumber will not work if the number is possible but not
        // valid. There is in fact one country calling code (290) where the possible
        // number pattern differs between various regions (Saint Helena and Tristan da
        // Cuñha), but this is handled by putting all possible lengths for any country
        // with this country calling code in the metadata for the default region in
        // this case.
        if !self.has_valid_country_calling_code(country_code) {
            return Err(ValidationError::InvalidCountryCode);
        }
        let region_code = self.get_region_code_for_country_code(country_code);
        // Metadata cannot be NULL because the country calling code is valid.
        let Some(metadata) =
            self.get_metadata_for_region_or_calling_code(country_code, region_code)
        else {
            return Err(ValidationError::InvalidCountryCode);
        };
        return test_number_length(&national_number, metadata, phone_number_type);
    }

    pub fn truncate_too_long_number(
        &self,
        phone_number: &mut PhoneNumber,
    ) -> InternalLogicResult<bool> {
        if self.is_valid_number(&phone_number)? {
            return Ok(true);
        }
        let mut number_copy = phone_number.clone();
        let mut national_number = phone_number.national_number();
        loop {
            national_number /= 10;
            number_copy.set_national_number(national_number);
            if self
                .is_possible_number_with_reason(&number_copy)
                .is_err_and(|err| matches!(err, ValidationError::TooShort))
                || national_number == 0
            {
                return Ok(false);
            }
            if self.is_valid_number(&number_copy)? {
                break;
            }
        }
        phone_number.set_national_number(national_number);
        return Ok(true);
    }

    // Note if any new field is added to this method that should always be filled
    // in, even when keepRawInput is false, it should also be handled in the
    // CopyCoreFieldsOnly() method.
    pub fn parse_helper(
        &self,
        number_to_parse: &str,
        default_region: &str,
        keep_raw_input: bool,
        check_region: bool,
    ) -> ParseResult<PhoneNumber> {
        let national_number = self.build_national_number_for_parsing(number_to_parse)?;
        if !self.is_viable_phone_number(&national_number) {
            trace!("The string supplied did not seem to be a phone number '{national_number}'.");
            return Err(ParseError::NotANumber(NotANumberError::NotMatchedValidNumberPattern));
        }

        if check_region && !self.check_region_for_parsing(&national_number, default_region) {
            trace!("Missing or invalid default country.");
            return Err(ParseError::InvalidCountryCode);
        }
        let mut temp_number = PhoneNumber::new();
        if keep_raw_input {
            temp_number.set_raw_input(number_to_parse.to_owned());
        }
        // Attempt to parse extension first, since it doesn't require country-specific
        // data and we want to have the non-normalised number here.

        let (national_number, extension) = self.maybe_strip_extension(&national_number);

        if let Some(extension) = extension {
            temp_number.set_extension(extension.to_owned());
        }
        let mut country_metadata = self.get_metadata_for_region(default_region);
        // Check to see if the number is given in international format so we know
        // whether this number is from the default country or not.
        let mut normalized_national_number = self
            .maybe_extract_country_code(
                country_metadata,
                keep_raw_input,
                &national_number,
                &mut temp_number,
            )
            .or_else(|err| {
                if !matches!(err, ParseError::InvalidCountryCode) {
                    return Err(err);
                }
                let plus_match = self.reg_exps.plus_chars_pattern.find_start(national_number);
                if let Some(plus_match) = plus_match {
                    let normalized_national_number = &national_number[plus_match.end()..];
                    // Strip the plus-char, and try again.
                    let normalized_national_number = self.maybe_extract_country_code(
                        country_metadata,
                        keep_raw_input,
                        normalized_national_number,
                        &mut temp_number,
                    )?;
                    if temp_number.country_code() == 0 {
                        return Err(ParseError::InvalidCountryCode.into());
                    }
                    return Ok(normalized_national_number);
                }
                Err(err)
            })?;

        let mut country_code = temp_number.country_code();
        if country_code != 0 {
            let phone_number_region = self.get_region_code_for_country_code(country_code);
            if phone_number_region != default_region {
                country_metadata =
                    self.get_metadata_for_region_or_calling_code(country_code, phone_number_region);
            }
        } else if let Some(country_metadata) = country_metadata {
            // If no extracted country calling code, use the region supplied instead.
            // Note that the national number was already normalized by
            // MaybeExtractCountryCode.
            country_code = country_metadata.country_code();
        }
        if normalized_national_number.len() < MIN_LENGTH_FOR_NSN {
            trace!(
                "The string supplied is too short to be a phone number '{}'.",
                normalized_national_number
            );
            return Err(ParseError::TooShortNsn.into());
        }
        if let Some(country_metadata) = country_metadata {
            let mut potential_national_number = normalized_national_number.clone();

            let (phone_number, carrier_code) = self.maybe_strip_national_prefix_and_carrier_code(
                country_metadata,
                &potential_national_number,
            )?;

            let carrier_code = carrier_code
                .map(|c| c.to_string());

            if potential_national_number != phone_number {
                potential_national_number = Cow::Owned(phone_number.into_owned());
            }

            // We require that the NSN remaining after stripping the national prefix
            // and carrier code be long enough to be a possible length for the region.
            // Otherwise, we don't do the stripping, since the original number could be
            // a valid short number.
            let validation_result =
                test_number_length_with_unknown_type(&potential_national_number, country_metadata);
            if !validation_result
                .is_ok_and(|res| matches!(res, NumberLengthType::IsPossibleLocalOnly))
                && !validation_result.is_err_and(|err| {
                    matches!(
                        err,
                        ValidationError::TooShort | ValidationError::InvalidLength
                    )
                })
            {
                normalized_national_number = potential_national_number;
                if let Some(carrier_code) = carrier_code.filter(|_| keep_raw_input) {
                    temp_number.set_preferred_domestic_carrier_code(carrier_code.to_owned());
                }
            }
        }
        let normalized_national_number_length = normalized_national_number.len();
        if normalized_national_number_length < MIN_LENGTH_FOR_NSN {
            trace!(
                "The string supplied is too short to be a phone number: '{}'.",
                normalized_national_number
            );
            return Err(ParseError::TooShortNsn.into());
        }
        if normalized_national_number_length > MAX_LENGTH_FOR_NSN {
            trace!(
                "The string supplied is too long to be a phone number: '{}'.",
                normalized_national_number
            );
            return Err(ParseError::TooLongNsn.into());
        }
        temp_number.set_country_code(country_code);

        if let Some(zeroes_count) =
            Self::get_italian_leading_zeros_for_phone_number(&normalized_national_number) {
            temp_number.set_italian_leading_zero(true);
            if zeroes_count > 1 {
                temp_number.set_number_of_leading_zeros(zeroes_count as i32);
            }
        }
        let number_as_int = u64::from_str_radix(&normalized_national_number, 10);
        match number_as_int {
            Ok(number_as_int) => temp_number.set_national_number(number_as_int),
            Err(err) => return Err(NotANumberError::FailedToParseNumberAsInt(err).into()),
        }
        return Ok(temp_number);
    }

    /// Checks to see if the string of characters could possibly be a phone number at
    /// all. At the moment, checks to see that the string begins with at least 3
    /// digits, ignoring any punctuation commonly found in phone numbers.  This
    /// method does not require the number to be normalized in advance - but does
    /// assume that leading non-number symbols have been removed, such as by the
    /// method `ExtractPossibleNumber`.
    pub fn is_viable_phone_number(&self, phone_number: &str) -> bool {
        if phone_number.len() < MIN_LENGTH_FOR_NSN {
            false
        } else {
            self.reg_exps
                .valid_phone_number_pattern
                .full_match(phone_number)
        }
    }

    /// Checks to see that the region code used is valid, or if it is not valid, that
    /// the number to parse starts with a + symbol so that we can attempt to infer
    /// the country from the number. Returns false if it cannot use the region
    /// provided and the region cannot be inferred.
    pub fn check_region_for_parsing(&self, number_to_parse: &str, default_region: &str) -> bool {
        self.get_metadata_for_region(default_region).is_some()
            || number_to_parse.is_empty()
            || self
                .reg_exps
                .plus_chars_pattern
                .matches_start(number_to_parse)
    }

    /// Strips any extension (as in, the part of the number dialled after the call is
    /// connected, usually indicated with extn, ext, x or similar) from the end of
    /// the number, and returns stripped number and extension. The number passed in should be non-normalized.
    pub fn maybe_strip_extension<'a>(&self, phone_number: &'a str) -> (&'a str, Option<&'a str>) {
        let Some(captures) = self.reg_exps.extn_pattern.captures(phone_number) else {
            return (phone_number, None);
        };

        // first capture is always not None, this should not happen, but use this for safety.
        let Some(full_capture) = captures.get(0) else {
            return (phone_number, None);
        };
        // Replace the extensions in the original string here.
        let phone_number_no_extn = &phone_number[..full_capture.start()];
        // If we find a potential extension, and the number preceding this is a
        // viable number, we assume it is an extension.
        if !self.is_viable_phone_number(&phone_number_no_extn) {
            return (phone_number, None);
        }
        if let Some(ext) = captures.iter().skip(1).flatten().find(|m| !m.is_empty()) {
            return (phone_number_no_extn, Some(ext.as_str()));
        }

        (phone_number, None)
    }

    /// Tries to extract a country calling code from a number. Country calling codes
    /// are extracted in the following ways:
    ///   - by stripping the international dialing prefix of the region the person
    ///   is dialing from, if this is present in the number, and looking at the next
    ///   digits
    ///   - by stripping the '+' sign if present and then looking at the next digits
    ///   - by comparing the start of the number and the country calling code of the
    ///   default region. If the number is not considered possible for the numbering
    ///   plan of the default region initially, but starts with the country calling
    ///   code of this region, validation will be reattempted after stripping this
    ///   country calling code. If this number is considered a possible number, then
    ///   the first digits will be considered the country calling code and removed as
    ///   such.
    ///
    ///   Returns `Ok` if a country calling code was successfully
    ///   extracted or none was present, or the appropriate error otherwise, such as
    ///   if a + was present but it was not followed by a valid country calling code.
    ///   If NO_PARSING_ERROR is returned, the national_number without the country
    ///   calling code is populated, and the country_code of the phone_number passed
    ///   in is set to the country calling code if found, otherwise to 0.
    pub fn maybe_extract_country_code<'a>(
        &self,
        default_region_metadata: Option<&PhoneMetadata>,
        keep_raw_input: bool,
        national_number: &'a str,
        phone_number: &mut PhoneNumber,
    ) -> ParseResult<Cow<'a, str>> {
        // Set the default prefix to be something that will never match if there is no
        // default region.
        let possible_country_idd_prefix =
            if let Some(default_region_metadata) = default_region_metadata {
                default_region_metadata.international_prefix()
            } else {
                "NonMatch"
            };

        let phone_number_with_country_code_source = self
            .maybe_strip_international_prefix_and_normalize(
                national_number,
                possible_country_idd_prefix,
            )?;

        let national_number = phone_number_with_country_code_source.phone_number;
        if keep_raw_input {
            phone_number
                .set_country_code_source(phone_number_with_country_code_source.country_code_source);
        }
        if !matches!(
            phone_number_with_country_code_source.country_code_source,
            CountryCodeSource::FROM_DEFAULT_COUNTRY
        ) {
            if national_number.len() <= MIN_LENGTH_FOR_NSN {
                trace!(
                    "Phone number {} had an IDD, but after this was not \
                long enough to be a viable phone number.",
                    national_number
                );
                return Err(ParseError::TooShortAfterIdd);
            }
            let Some((national_number, potential_country_code)) =
                self.extract_country_code(national_number)
            else {
                // If this fails, they must be using a strange country calling code that we
                // don't recognize, or that doesn't exist.
                return Err(ParseError::InvalidCountryCode);
            };
            phone_number.set_country_code(potential_country_code);
            return Ok(national_number);
        } else if let Some(default_region_metadata) = default_region_metadata {
            // Check to see if the number starts with the country calling code for the
            // default region. If so, we remove the country calling code, and do some
            // checks on the validity of the number before and after.
            let default_country_code = default_region_metadata.country_code();
            let mut buf = itoa::Buffer::new();
            let default_country_code_string = buf.format(default_country_code);
            trace!(
                "Possible country calling code for number '{}': {}",
                national_number, default_country_code_string
            );
            if let Some(potential_national_number) =
                strip_cow_prefix(national_number.clone(), default_country_code_string)
            {
                let general_num_desc = &default_region_metadata.general_desc;
                let phone_number_and_carrier_code = self
                    .maybe_strip_national_prefix_and_carrier_code(
                        default_region_metadata,
                        &potential_national_number,
                    )?;

                trace!(
                    "Number without country calling code prefix: {:?}",
                    phone_number_and_carrier_code
                );
                // If the number was not valid before but is valid now, or if it was too
                // long before, we consider the number with the country code stripped to
                // be a better result and keep that instead.
                if (!helper_functions::is_match(
                    &self.matcher_api,
                    &national_number,
                    general_num_desc,
                ) && helper_functions::is_match(
                    &self.matcher_api,
                    &potential_national_number,
                    general_num_desc,
                )) || test_number_length_with_unknown_type(
                    &national_number,
                    default_region_metadata,
                )
                .is_err_and(|e| matches!(e, ValidationError::TooLong))
                {
                    if keep_raw_input {
                        phone_number.set_country_code_source(
                            CountryCodeSource::FROM_NUMBER_WITHOUT_PLUS_SIGN,
                        );
                    }
                    phone_number.set_country_code(default_country_code);
                    return Ok(potential_national_number);
                }
            }
        }
        // No country calling code present. Set the country_code to 0.
        phone_number.set_country_code(0);
        return Ok(national_number);
    }

    /// Gets a valid fixed-line number for the specified region_code. Returns false
    /// if no number exists.
    pub fn get_example_number(&self, region_code: &str) -> ExampleNumberResult {
        self.get_example_number_for_type_and_region_code(region_code, PhoneNumberType::FixedLine)
    }

    pub fn get_invalid_example_number(&self, region_code: &str) -> ExampleNumberResult {
        let Some(region_metadata) = self.region_to_metadata_map.get(region_code) else {
            warn!("Invalid or unknown region code ({}) provided.", region_code);
            return Err(GetExampleNumberError::InvalidMetadata);
        };

        // We start off with a valid fixed-line number since every country supports
        // this. Alternatively we could start with a different number type, since
        // fixed-line numbers typically have a wide breadth of valid number lengths
        // and we may have to make it very short before we get an invalid number.
        let desc = get_number_desc_by_type(region_metadata, PhoneNumberType::FixedLine);

        if !desc.has_example_number() {
            // This shouldn't happen - we have a test for this.
            return Err(GetExampleNumberError::NoExampleNumber);
        }

        let example_number = desc.example_number();
        // Try and make the number invalid. We do this by changing the length. We try
        // reducing the length of the number, since currently no region has a number
        // that is the same length as kMinLengthForNsn. This is probably quicker than
        // making the number longer, which is another alternative. We could also use
        // the possible number pattern to extract the possible lengths of the number
        // to make this faster, but this method is only for unit-testing so simplicity
        // is preferred to performance.
        // We don't want to return a number that can't be parsed, so we check the
        // number is long enough. We try all possible lengths because phone number
        // plans often have overlapping prefixes so the number 123456 might be valid
        // as a fixed-line number, and 12345 as a mobile number. It would be faster to
        // loop in a different order, but we prefer numbers that look closer to real
        // numbers (and it gives us a variety of different lengths for the resulting
        // phone numbers - otherwise they would all be kMinLengthForNsn digits long.)
        for phone_number_length in
            (MIN_LENGTH_FOR_NSN..=example_number.len().saturating_sub(1)).rev()
        {
            let number_to_try = &example_number[0..phone_number_length];
            let Ok(possibly_valid_number) = self.parse(&number_to_try, region_code) else {
                continue;
            };
            // We don't check the return value since we have already checked the
            // length, we know example numbers have only valid digits, and we know the
            // region code is fine.
            if !self.is_valid_number(&possibly_valid_number)? {
                return Ok(possibly_valid_number);
            }
        }
        // We have a test to check that this doesn't happen for any of our supported
        Err(GetExampleNumberError::CouldNotGetNumber)
    }

    // Gets a valid number for the specified region_code and type.  Returns false if
    // no number exists.
    pub fn get_example_number_for_type_and_region_code(
        &self,
        region_code: &str,
        phone_number_type: PhoneNumberType,
    ) -> ExampleNumberResult {
        let Some(region_metadata) = self.region_to_metadata_map.get(region_code) else {
            warn!("Invalid or unknown region code ({}) provided.", region_code);
            return Err(GetExampleNumberError::InvalidMetadata);
        };
        let desc = get_number_desc_by_type(region_metadata, phone_number_type);
        if desc.has_example_number() {
            return Ok(self
                .parse(desc.example_number(), region_code)
                .inspect_err(|err| error!("Error parsing example number ({:?})", err))?);
        }
        Err(GetExampleNumberError::CouldNotGetNumber)
    }

    pub fn get_example_number_for_type(
        &self,
        phone_number_type: PhoneNumberType,
    ) -> ExampleNumberResult {
        if let Some(number) = self.get_supported_regions().find_map(|region_code| {
            self.get_example_number_for_type_and_region_code(region_code, phone_number_type)
                .ok()
        }) {
            return Ok(number);
        }

        // If there wasn't an example number for a region, try the non-geographical
        // entities.
        if let Some(res) = self
            .get_supported_global_network_calling_codes()
            .into_iter()
            .find_map(|country_calling_code| {
                let Some(metadata) = self
                    .country_code_to_non_geographical_metadata_map
                    .get(&country_calling_code)
                else {
                    return Some(Err(GetExampleNumberError::InvalidMetadata));
                };
                let desc = get_number_desc_by_type(metadata, phone_number_type);
                if desc.has_example_number() {
                    let mut buf = itoa::Buffer::new();
                    return Some(
                        self.parse(
                            &fast_cat::concat_str!(
                                PLUS_SIGN,
                                buf.format(country_calling_code),
                                desc.example_number()
                            ),
                            RegionCode::get_unknown(),
                        )
                        .map_err(|err| GetExampleNumberError::FailedToParse(err)),
                    );
                }
                None
            })
        {
            return res;
        }
        // There are no example numbers of this type for any country in the library.
        Err(GetExampleNumberError::CouldNotGetNumber)
    }

    pub fn get_example_number_for_non_geo_entity(
        &self,
        country_calling_code: i32,
    ) -> ExampleNumberResult {
        let Some(metadata) = self
            .country_code_to_non_geographical_metadata_map
            .get(&country_calling_code)
        else {
            warn!(
                "Invalid or unknown country calling code provided: {}",
                country_calling_code
            );
            return Err(GetExampleNumberError::InvalidMetadata);
        };
        // For geographical entities, fixed-line data is always present. However,
        // for non-geographical entities, this is not the case, so we have to go
        // through different types to find the example number. We don't check
        // fixed-line or personal number since they aren't used by non-geographical
        // entities (if this changes, a unit-test will catch this.)
        const NUMBER_TYPES_COUNT: usize = 7;

        let types: [_; NUMBER_TYPES_COUNT] = [
            &metadata.mobile,
            &metadata.toll_free,
            &metadata.shared_cost,
            &metadata.voip,
            &metadata.voicemail,
            &metadata.uan,
            &metadata.premium_rate,
        ];
        for number_type in types {
            if !number_type.has_example_number() {
                continue;
            }
            let mut buf = itoa::Buffer::new();
            return Ok(self.parse(
                &fast_cat::concat_str!(
                    PLUS_SIGN,
                    buf.format(country_calling_code),
                    number_type.example_number()
                ),
                RegionCode::get_unknown(),
            )?);
        }
        return Err(GetExampleNumberError::CouldNotGetNumber);
    }

    /// Strips any international prefix (such as +, 00, 011) present in the number
    /// provided, normalizes the resulting number, and indicates if an international
    /// prefix was present.
    ///
    /// possible_idd_prefix represents the international direct dialing prefix from
    /// the region we think this number may be dialed in.
    /// Returns true if an international dialing prefix could be removed from the
    /// number, otherwise false if the number did not seem to be in international
    /// format.
    pub fn maybe_strip_international_prefix_and_normalize<'a>(
        &self,
        phone_number: &'a str,
        possible_idd_prefix: &str,
    ) -> RegexResult<PhoneNumberWithCountryCodeSource<'a>> {
        if phone_number.is_empty() {
            Ok(PhoneNumberWithCountryCodeSource::new(
                Cow::Borrowed(phone_number),
                CountryCodeSource::FROM_DEFAULT_COUNTRY,
            ))
        } else if let Some(plus_match) =
            self.reg_exps.plus_chars_pattern.find_start(phone_number)
        {
            let number_string_piece = &phone_number[plus_match.end()..];
            // Can now normalize the rest of the number since we've consumed the "+"
            // sign at the start.
            Ok(PhoneNumberWithCountryCodeSource::new(
                Cow::Owned(self.normalize(number_string_piece)),
                CountryCodeSource::FROM_NUMBER_WITH_PLUS_SIGN,
            ))
        } else {
            // Attempt to parse the first digits as an international prefix.
            let idd_pattern = self.reg_exps.regexp_cache.get_regex(possible_idd_prefix)?;
            let normalized_number = self.normalize(phone_number);
            let value = if let Some(stripped_prefix_number) =
                self.parse_prefix_as_idd(&normalized_number, idd_pattern)
            {
                PhoneNumberWithCountryCodeSource::new(
                    Cow::Owned(stripped_prefix_number.to_owned()),
                    CountryCodeSource::FROM_NUMBER_WITH_IDD,
                )
            } else {
                PhoneNumberWithCountryCodeSource::new(
                    Cow::Owned(normalized_number),
                    CountryCodeSource::FROM_DEFAULT_COUNTRY,
                )
            };

            Ok(value)
        }
    }

    /// Normalizes a string of characters representing a phone number. This performs
    /// the following conversions:
    ///   - Punctuation is stripped.
    ///   For ALPHA/VANITY numbers:
    ///   - Letters are converted to their numeric representation on a telephone
    ///     keypad. The keypad used here is the one defined in ITU Recommendation
    ///     E.161. This is only done if there are 3 or more letters in the number, to
    ///     lessen the risk that such letters are typos.
    ///   For other numbers:
    ///   - Wide-ascii digits are converted to normal ASCII (European) digits.
    ///   - Arabic-Indic numerals are converted to European numerals.
    ///   - Spurious alpha characters are stripped.
    pub fn normalize(&self, phone_number: &str) -> String {
        if self
            .reg_exps
            .valid_alpha_phone_pattern
            .is_match(phone_number)
        {
            normalize_helper(&self.reg_exps.alpha_phone_mappings, true, phone_number)
        } else {
            self.normalize_digits_only(phone_number)
        }
    }

    /// Strips the IDD from the start of the number if present. Helper function used
    /// by MaybeStripInternationalPrefixAndNormalize.
    pub fn parse_prefix_as_idd<'a>(
        &self,
        phone_number: &'a str,
        idd_pattern: Arc<Regex>,
    ) -> Option<&'a str> {
        // First attempt to strip the idd_pattern at the start, if present. We make a
        // copy so that we can revert to the original string if necessary.
        let Some(idd_pattern_match) = idd_pattern.find_start(&phone_number) else {
            return None;
        };
        let captured_range_end = idd_pattern_match.end();

        // Only strip this if the first digit after the match is not a 0, since
        // country calling codes cannot begin with 0.
        if phone_number[captured_range_end..]
            .chars()
            .find(|c| c.is_decimal_utf8())
            .and_then(|c| c.to_decimal_utf8())
            == Some(0)
        {
            return None;
        }
        Some(&phone_number[captured_range_end..])
    }

    pub fn is_number_geographical(&self, phone_number: &PhoneNumber) -> InternalLogicResult<bool> {
        Ok(self.is_number_geographical_by_country_code_and_type(
            self.get_number_type(phone_number)?,
            phone_number.country_code(),
        ))
    }

    pub fn is_number_geographical_by_country_code_and_type(
        &self,
        phone_number_type: PhoneNumberType,
        country_calling_code: i32,
    ) -> bool {
        matches!(
            phone_number_type,
            PhoneNumberType::FixedLine | PhoneNumberType::FixedLineOrMobile
        ) || (self
            .reg_exps
            .geo_mobile_countries
            .contains(&country_calling_code)
            && matches!(phone_number_type, PhoneNumberType::Mobile))
    }

    pub fn get_length_of_geographical_area_code(
        &self,
        phone_number: &PhoneNumber,
    ) -> InternalLogicResult<usize> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) = self.region_to_metadata_map.get(region_code) else {
            return Ok(0);
        };

        let phone_number_type = self.get_number_type(phone_number)?;
        let country_calling_code = phone_number.country_code();

        // If a country doesn't use a national prefix, and this number doesn't have an
        // Italian leading zero, we assume it is a closed dialling plan with no area
        // codes.
        // Note:this is our general assumption, but there are exceptions which are
        // tracked in COUNTRIES_WITHOUT_NATIONAL_PREFIX_WITH_AREA_CODES.
        if !metadata.has_national_prefix()
            && !phone_number.italian_leading_zero()
            && !self
                .reg_exps
                .countries_without_national_prefix_with_area_codes
                .contains(&country_calling_code)
        {
            return Ok(0);
        }

        if (matches!(phone_number_type, PhoneNumberType::Mobile)
            && self
                .reg_exps
                .geo_mobile_countries_without_mobile_area_codes
                .contains(&country_calling_code)) {
            return Ok(0);
        }

        if !self.is_number_geographical_by_country_code_and_type(
            phone_number_type,
            country_calling_code,
        ) {
            return Ok(0);
        }

        return self.get_length_of_national_destination_code(phone_number);
    }

    pub fn get_length_of_national_destination_code(
        &self,
        phone_number: &PhoneNumber,
    ) -> InternalLogicResult<usize> {
        let mut copied_proto = phone_number.clone();
        if phone_number.has_extension() {
            // Clear the extension so it's not included when formatting.
            copied_proto.clear_extension();
        }

        let formatted_number = self.format(&copied_proto, PhoneNumberFormat::International)?;

        const ITERATIONS_COUNT: usize = 3;
        let mut captured_groups = [0; ITERATIONS_COUNT];
        let (ndc_index, third_group) = (1, 2);
        let mut capture_iter = self
            .reg_exps
            .capturing_ascii_digits_pattern
            .captures_iter(&formatted_number);
        for i in 0..ITERATIONS_COUNT {
            if let Some(matches) = capture_iter.next().and_then(|captures| captures.get(1)) {
                captured_groups[i] = matches.len();
            } else {
                return Ok(0);
            }
        }

        if matches!(self.get_number_type(phone_number)?, PhoneNumberType::Mobile) {
            // For example Argentinian mobile numbers, when formatted in the
            // international format, are in the form of +54 9 NDC XXXX.... As a result,
            // we take the length of the third group (NDC) and add the length of the
            // mobile token, which also forms part of the national significant number.
            // This assumes that the mobile token is always formatted separately from
            // the rest of the phone number.
            if let Some(mobile_token) = self.get_country_mobile_token(phone_number.country_code()) {
                return Ok(captured_groups[third_group] + mobile_token.len_utf8());
            }
        }
        Ok(captured_groups[ndc_index])
    }

    pub fn get_country_mobile_token(&self, country_calling_code: i32) -> Option<char> {
        self.reg_exps
            .mobile_token_mappings
            .get(&country_calling_code)
            .copied()
    }

    /// Extracts country calling code from national_number, and returns tuple
    /// that contains national_number without calling code and calling code itself.
    ///
    /// It assumes that the leading plus sign or IDD has already been removed.
    ///
    /// Returns None if national_number doesn't start with a valid country calling code
    /// Assumes the national_number is at least 3 characters long.
    pub fn extract_country_code<'a>(
        &self,
        national_number: Cow<'a, str>,
    ) -> Option<(Cow<'a, str>, i32)> {
        if national_number.as_ref().is_empty() || national_number.as_ref().starts_with('0') {
            return None;
        }
        for i in 0..=MAX_LENGTH_COUNTRY_CODE {
            let Ok(potential_country_code) =
                i32::from_str_radix(&national_number.as_ref()[0..i], 10)
            else {
                continue;
            };
            let region_code = self.get_region_code_for_country_code(potential_country_code);
            if region_code != RegionCode::get_unknown() {
                return match national_number {
                    Cow::Borrowed(s) => Some((Cow::Borrowed(&s[i..]), potential_country_code)),
                    Cow::Owned(mut s) => {
                        s.drain(0..i);
                        Some((Cow::Owned(s), potential_country_code))
                    }
                };
            }
        }
        return None;
    }

    // Strips any national prefix (such as 0, 1) present in the number provided.
    // The number passed in should be the normalized telephone number that we wish
    // to strip any national dialing prefix from. The metadata should be for the
    // region that we think this number is from. Returns true if a national prefix
    // and/or carrier code was stripped.
    pub fn maybe_strip_national_prefix_and_carrier_code<'a>(
        &self,
        metadata: &PhoneMetadata,
        phone_number: &'a str,
    ) -> RegexResult<(Cow<'a, str>, Option<&'a str>)> {
        let possible_national_prefix = metadata.national_prefix_for_parsing();
        if phone_number.is_empty() || possible_national_prefix.is_empty() {
            // Early return for numbers of zero length or with no national prefix
            // possible.
            return Ok((phone_number.into(), None));
        }
        let general_desc = &metadata.general_desc;
        // Check if the original number is viable.
        let is_viable_original_number =
            helper_functions::is_match(&self.matcher_api, &phone_number, general_desc);
        // Attempt to parse the first digits as a national prefix. We make a
        // copy so that we can revert to the original string if necessary.
        let transform_rule = metadata.national_prefix_transform_rule();

        let possible_national_prefix_pattern = self
            .reg_exps
            .regexp_cache
            .get_regex(possible_national_prefix)?;

        let captures = possible_national_prefix_pattern.captures_start(&phone_number);
        let first_capture = captures.as_ref().and_then(|c| c.get(1));
        let second_capture = captures.as_ref().and_then(|c| c.get(2));

        let condition = |first_capture: &regex::Match<'_>| {
            !transform_rule.is_empty()
                && (second_capture.is_some_and(|c| !c.is_empty())
                    || !first_capture.is_empty() && second_capture.is_none())
        };

        if let Some(first_capture) = first_capture.filter(condition) {
            // here we can safe unwrap because first_capture.is_some() anyway
            let carrier_code_temp = if second_capture.is_some() {
                Some(first_capture.as_str())
            } else {
                None
            };

            // If this succeeded, then we must have had a transform rule and there must
            // have been some part of the prefix that we captured.
            // We make the transformation and check that the resultant number is still
            // viable. If so, replace the number and return.

            // Rust note: There is no known transform rules containing $\d\d 
            // But if any appears this should be handled with {} braces: {$\d}\d
            let replaced_number =
                possible_national_prefix_pattern.replace(&phone_number, transform_rule);
            if is_viable_original_number
                && !helper_functions::is_match(&self.matcher_api, &replaced_number, general_desc)
            {
                return Ok((phone_number.into(), None));
            }
            return Ok((replaced_number, carrier_code_temp));
        } else if let Some(matched) = captures.and_then(|c| c.get(0)) {
            trace!(
                "Parsed the first digits as a national prefix for number '{}'.",
                phone_number
            );
            // If captured_part_of_prefix is empty, this implies nothing was captured by
            // the capturing groups in possible_national_prefix; therefore, no
            // transformation is necessary, and we just remove the national prefix.
            let stripped_number = &phone_number[matched.end()..];
            if is_viable_original_number
                && !helper_functions::is_match(&self.matcher_api, stripped_number, general_desc) {
                return Ok((phone_number.into(), None));
            }
            let carrier_code_temp = if let Some(capture) = first_capture {
                Some(capture.as_str())
            } else {
                None
            };

            return Ok((stripped_number.into(), carrier_code_temp));
        }
        trace!(
            "The first digits did not match the national prefix for number '{}'.",
            phone_number
        );
        Ok((phone_number.into(), None))
    }

    // A helper function to set the values related to leading zeros in a
    // PhoneNumber.
    pub fn get_italian_leading_zeros_for_phone_number(national_number: &str) -> Option<usize> {
        if national_number.len() < 2 {
            return None;
        }
        let zero_count = national_number.chars().take_while(|c| *c == '0').count();
        if zero_count == 0 {
            return None
        }
        // Note that if the national number is all "0"s, the last "0" is not
        // counted as a leading zero.
        if zero_count == national_number.len() {
            return Some(zero_count - 1);
        }

        Some(zero_count)
    }

    pub fn convert_alpha_characters_in_number(&self, phone_number: &str) -> String {
        normalize_helper(&self.reg_exps.alpha_phone_mappings, false, phone_number)
    }

    pub fn is_number_match(
        &self,
        first_number_in: &PhoneNumber,
        second_number_in: &PhoneNumber,
    ) -> MatchType {
        // Early exit if both had extensions and these are different.
        if first_number_in.has_extension()
            && second_number_in.has_extension()
            && first_number_in.extension() != second_number_in.extension()
        {
            return MatchType::NoMatch;
        }

        // We only are about the fields that uniquely define a number, so we copy
        // these across explicitly.
        let mut first_number = copy_core_fields_only(&first_number_in);
        let second_number = copy_core_fields_only(&second_number_in);

        let first_number_country_code = first_number.country_code();
        let second_number_country_code = second_number.country_code();
        // Both had country calling code specified.
        if first_number_country_code != 0 && second_number_country_code != 0 {
            if first_number == second_number {
                return MatchType::ExactMatch;
            } else if first_number_country_code == second_number_country_code
                && is_national_number_suffix_of_the_other(&first_number, &second_number)
            {
                // A SHORT_NSN_MATCH occurs if there is a difference because of the
                // presence or absence of an 'Italian leading zero', the presence or
                // absence of an extension, or one NSN being a shorter variant of the
                // other.
                return MatchType::ShortNsnMatch;
            }
            // This is not a match.
            return MatchType::NoMatch;
        }
        // Checks cases where one or both country calling codes were not specified. To
        // make equality checks easier, we first set the country_code fields to be
        // equal.
        first_number.set_country_code(second_number_country_code);
        // If all else was the same, then this is an NSN_MATCH.
        if first_number == second_number {
            return MatchType::NsnMatch;
        }
        if is_national_number_suffix_of_the_other(&first_number, &second_number) {
            return MatchType::ShortNsnMatch;
        }
        return MatchType::NoMatch;
    }

    pub fn is_number_match_with_two_strings(
        &self,
        first_number: &str,
        second_number: &str,
    ) -> MatchResult {
        match self.parse(first_number, RegionCode::get_unknown()) {
            Ok(first_number_as_proto) => {
                return self.is_number_match_with_one_string(&first_number_as_proto, second_number);
            }
            Err(err) => {
                if !matches!(err, ParseError::InvalidCountryCode) {
                    return Err(InvalidNumberError(err));
                }
            }
        }
        match self.parse(second_number, RegionCode::get_unknown()) {
            Ok(second_number_as_proto) => {
                return self.is_number_match_with_one_string(&second_number_as_proto, first_number);
            }
            Err(err) => {
                if !matches!(err, ParseError::InvalidCountryCode) {
                    return Err(InvalidNumberError(err));
                }
                let first_number_as_proto =
                    self.parse_helper(first_number, RegionCode::get_unknown(), false, false)?;
                let second_number_as_proto = self.parse_helper(
                    second_number,
                    RegionCode::get_unknown(),
                    false,
                    false,
                )?;
                return Ok(self.is_number_match(&first_number_as_proto, &second_number_as_proto));
            }
        }
    }

    pub fn is_number_match_with_one_string(
        &self,
        first_number: &PhoneNumber,
        second_number: &str,
    ) -> MatchResult {
        // First see if the second number has an implicit country calling code, by
        // attempting to parse it.
        match self.parse(second_number, RegionCode::get_unknown()) {
            Ok(second_number_as_proto) => {
                return Ok(self.is_number_match(first_number, &second_number_as_proto));
            }
            Err(err) => {
                if !matches!(err, ParseError::InvalidCountryCode) {
                    return Err(InvalidNumberError(err));
                }
            }
        }
        // The second number has no country calling code. EXACT_MATCH is no longer
        // possible.  We parse it as if the region was the same as that for the
        // first number, and if EXACT_MATCH is returned, we replace this with
        // NSN_MATCH.
        let first_number_region =
            self.get_region_code_for_country_code(first_number.country_code());
        if first_number_region != RegionCode::get_unknown() {
            let second_number_with_first_number_region =
                self.parse(second_number, first_number_region)?;
            return Ok(
                match self.is_number_match(first_number, &second_number_with_first_number_region) {
                    MatchType::ExactMatch => MatchType::NsnMatch,
                    m => m,
                },
            );
        } else {
            // If the first number didn't have a valid country calling code, then we
            // parse the second number without one as well.
            let second_number_as_proto =
                self.parse_helper(second_number, RegionCode::get_unknown(), false, false)?;
            return Ok(self.is_number_match(first_number, &second_number_as_proto));
        }
    }

    pub fn is_alpha_number(&self, phone_number: &str) -> bool {
        if !self.is_viable_phone_number(phone_number) {
            // Number is too short, or doesn't match the basic phone number pattern.
            return false;
        }
        // Copy the number, since we are going to try and strip the extension from it.
        let (number, _extension) = self.maybe_strip_extension(&phone_number);
        return self.reg_exps.valid_alpha_phone_pattern.full_match(number);
    }
}
