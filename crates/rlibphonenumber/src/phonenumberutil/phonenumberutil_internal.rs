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
    borrow::Cow,
    collections::{HashSet, VecDeque},
};

use super::{
    enums::{MatchType, NumberLengthType, PhoneNumberFormat, PhoneNumberType},
    errors::{
        ExtractNumberError, GetExampleNumberError, NotANumberError, ParseError, ValidationError,
    },
    helper_constants::{
        DEFAULT_EXTN_PREFIX, MAX_LENGTH_COUNTRY_CODE, MAX_LENGTH_FOR_NSN, MIN_LENGTH_FOR_NSN,
        NANPA_COUNTRY_CODE, PLUS_SIGN, REGION_CODE_FOR_NON_GEO_ENTITY, RFC3966_EXTN_PREFIX,
        RFC3966_ISDN_SUBADDRESS, RFC3966_PHONE_CONTEXT, RFC3966_PREFIX,
    },
    helper_functions::{
        self, copy_core_fields_only, get_number_desc_by_type,
        get_number_prefix_by_format_and_calling_code, get_supported_types_for_metadata,
        is_national_number_suffix_of_the_other, load_compiled_metadata, normalize_helper,
        test_number_length, test_number_length_with_unknown_type,
    },
    helper_types::PhoneNumberWithCountryCodeSource,
    phone_number_regexps_and_mappings::PhoneNumberRegExpsAndMappings,
};
use crate::{
    InternalError, InternalRegexError, InvalidNumberError,
    generated::{
        proto::{PhoneMetadataCollection, PhoneNumber, phone_number::CountryCodeSource},
        uniprops_digits, uniprops_without_nl,
    },
    phonenumberutil::{
        helper_constants::PLUS_CHARS,
        helper_functions::get_national_significant_number,
        helper_types::{PrefixParts, new_formatted_number_builder},
        regex_wrapper_types::{
            NumberFormatWrapper, PhoneMetadataWrapper, PhoneNumberDescWrapper, RegexTriplets,
        },
    },
    regex_based_matcher::RegexBasedMatcher,
    string_util::strip_cow_prefix,
};

use crate::regexp::Regex;
use log::{error, trace, warn};
use prost::DecodeError;
use rustc_hash::{FxHashMap, FxHashSet};

// Helper type for Result

pub type RegexResult<T> = std::result::Result<T, InternalRegexError>;

pub type ParseResult<T> = std::result::Result<T, InternalError<ParseError>>;

pub type ExampleNumberResult =
    std::result::Result<PhoneNumber, InternalError<GetExampleNumberError>>;
pub type ValidationResult = std::result::Result<NumberLengthType, ValidationError>;
pub type MatchResult = std::result::Result<MatchType, InternalError<InvalidNumberError>>;
pub type ExtractNumberResult<T> = std::result::Result<T, ExtractNumberError>;

pub struct PhoneNumberUtilInternal {
    /// Helper class holding useful regular expressions and character mappings.
    pub reg_exps: PhoneNumberRegExpsAndMappings,

    /// A mapping from a country calling code to a RegionCode object which denotes
    /// NANPA share the country calling code 1 and Russia and Kazakhstan share the
    /// country calling code 7. Under this map, 1 is mapped to region code "US" and
    /// 7 is mapped to region code "RU". This is implemented as a sorted vector to
    /// achieve better performance.
    country_calling_code_to_region_code_map: Vec<(i32, Vec<String>)>,

    nanpa_regions: FxHashSet<String>,

    /// A mapping from a region code to a PhoneMetadata for that region.
    region_to_metadata_map: FxHashMap<String, PhoneMetadataWrapper>,

    /// A mapping from a country calling code for a non-geographical entity to the
    /// PhoneMetadata for that country calling code. Examples of the country
    /// calling codes include 800 (International Toll Free Service) and 808
    /// (International Shared Cost Service).
    country_code_to_non_geographical_metadata_map: FxHashMap<i32, PhoneMetadataWrapper>,

    matcher_api: RegexBasedMatcher,
}

impl PhoneNumberUtilInternal {
    pub(crate) fn new_for_metadata(metadata_collection: PhoneMetadataCollection) -> Self {
        let mut instance = Self {
            reg_exps: PhoneNumberRegExpsAndMappings::new(),
            country_calling_code_to_region_code_map: Default::default(),
            nanpa_regions: Default::default(),
            matcher_api: Default::default(),
            region_to_metadata_map: Default::default(),
            country_code_to_non_geographical_metadata_map: Default::default(),
        };

        // that share a country calling code when inserting data.
        let mut country_calling_code_to_region_map = FxHashMap::<i32, VecDeque<String>>::default();
        for metadata in metadata_collection.metadata {
            let region_code = &metadata.id.clone();
            let main_country_code = metadata.main_country_for_code();

            if "ZZ" == region_code {
                continue;
            }

            let Some(country_calling_code) = metadata.country_code else {
                continue;
            };
            if REGION_CODE_FOR_NON_GEO_ENTITY == region_code {
                instance
                    .country_code_to_non_geographical_metadata_map
                    .insert(country_calling_code, metadata.into());
            } else {
                instance
                    .region_to_metadata_map
                    .insert(region_code.to_owned(), metadata.into());
            }

            let calling_code_in_map =
                country_calling_code_to_region_map.get_mut(&country_calling_code);
            if let Some(calling_code_in) = calling_code_in_map {
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

    /// Creates a new instance of the phone number utility.
    /// This method loads the compiled metadata for parsing, formatting, and validating phone numbers.
    ///
    /// You probably want use `PHONE_NUMBER_UTIL` singleton instead
    pub(crate) fn new() -> Result<Self, DecodeError> {
        let metadata_collection = load_compiled_metadata()?;
        Ok(Self::new_for_metadata(metadata_collection))
    }

    /// Gets an iterator over all region codes supported by the library.
    /// These are the regions for which metadata is available.
    pub(crate) fn get_supported_regions(&self) -> impl ExactSizeIterator<Item = &str> {
        self.region_to_metadata_map.keys().map(|k| k.as_str())
    }

    /// Gets an iterator over all supported global network calling codes.
    /// These are country codes for non-geographical entities, such as satellite services.
    pub(crate) fn get_supported_global_network_calling_codes(&self) -> impl Iterator<Item = i32> {
        self.country_code_to_non_geographical_metadata_map
            .keys()
            .copied()
    }

    /// Gets an iterator over all supported country calling codes.
    pub(crate) fn get_supported_calling_codes(&self) -> impl Iterator<Item = i32> {
        self.country_calling_code_to_region_code_map
            .iter()
            .map(|(k, _)| *k)
    }

    pub(crate) fn is_nanpa_country(&self, region_code: &str) -> bool {
        self.nanpa_regions.contains(region_code)
    }

    /// Gets a list of all supported phone number types for a given region.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region code for which to get the types.
    pub(crate) fn get_supported_types_for_region(
        &self,
        region_code: &str,
    ) -> Option<HashSet<PhoneNumberType>> {
        self.region_to_metadata_map
            .get(region_code)
            .map(get_supported_types_for_metadata)
            .or_else(|| {
                warn!("Invalid or unknown region code provided: {}", region_code);
                None
            })
    }

    /// Gets a list of all supported phone number types for a given non-geographical country calling code.
    ///
    /// # Arguments
    ///
    /// * `country_calling_code` - The non-geographical country calling code.
    pub(crate) fn get_supported_types_for_non_geo_entity(
        &self,
        country_calling_code: i32,
    ) -> Option<HashSet<PhoneNumberType>> {
        self.country_code_to_non_geographical_metadata_map
            .get(&country_calling_code)
            .map(get_supported_types_for_metadata)
            .or_else(|| {
                warn!(
                    "Unknown country calling code for a non-geographical entity provided: {}",
                    country_calling_code
                );
                None
            })
    }

    pub(crate) fn get_country_code_for_region(&self, region_code: &str) -> Option<i32> {
        self.region_to_metadata_map
            .get(region_code)
            .and_then(|metadata| metadata.original.country_code)
    }

    pub(crate) fn trim_unwanted_end_chars<'a>(&self, phone_number: &'a str) -> &'a str {
        phone_number.trim_end_matches(|c| {
            c != '#' && uniprops_without_nl::uniprops::Category::from_char(c).is_some()
        })
    }

    /// formatter is not implemented yet, but ..
    #[allow(unused)]
    pub(crate) fn is_format_eligible_for_as_you_type_formatter(&self, format: &str) -> bool {
        // We require that the first
        // group is present in the output pattern to ensure no data is lost while
        // formatting; when we format as you type, this should always be the case.
        self.reg_exps
            .is_format_eligible_as_you_type_formatting_regex_fullmatch
            .is_match(format)
    }

    #[allow(unused)]
    pub(crate) fn formatting_rule_has_first_group_only(
        &self,
        national_prefix_formatting_rule: &str,
    ) -> bool {
        national_prefix_formatting_rule.is_empty()
            || self
                .reg_exps
                .formatting_rule_has_first_group_only_regex_fullmatch
                .is_match(national_prefix_formatting_rule)
    }

    /// Gets the national direct dialing (NDD) prefix for a given region.
    /// This is the prefix used to make a national call within the region.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region code for which to get the NDD prefix.
    /// * `strip_non_digits` - If true, the returned prefix will contain only digits.
    pub(crate) fn get_ndd_prefix_for_region(
        &self,
        region_code: &str,
        strip_non_digits: bool,
    ) -> Option<String> {
        self.region_to_metadata_map
            .get(region_code)
            .map(|metadata| {
                let mut prefix = metadata.original.national_prefix().to_owned();
                if strip_non_digits {
                    prefix = prefix.replace("~", "");
                }
                prefix
            })
    }

    /// 'hot' function wrapper for region_to_metadata_map.get
    pub(crate) fn get_metadata_for_region(
        &self,
        region_code: &str,
    ) -> Option<&PhoneMetadataWrapper> {
        self.region_to_metadata_map.get(region_code)
    }

    /// Formats a phone number in the specified format.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to be formatted.
    /// * `number_format` - The format to be used.
    pub(crate) fn format<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        number_format: PhoneNumberFormat,
    ) -> RegexResult<Cow<'b, str>> {
        if phone_number.national_number == 0 {
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
        let country_calling_code = phone_number.country_code;
        let formatted_number_builder =
            new_formatted_number_builder(phone_number, Some(number_format));

        if matches!(number_format, PhoneNumberFormat::E164) {
            // Early exit for E164 case (even if the country calling code is invalid)
            // since no formatting of the national number needs to be applied.
            // Extensions are not formatted.
            return Ok(Cow::Owned(formatted_number_builder.build()?));
        }
        // Note here that all NANPA formatting rules are contained by US, so we use
        // rules are contained by Russia. French Indian Ocean country rules are
        // contained by Réunion.
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let metadata = region_code.and_then(|region_code| {
            self.get_metadata_for_region_or_calling_code(country_calling_code, region_code)
        });

        if let Some(metadata) = metadata {
            let formatted_number_builder = formatted_number_builder
                .with_format_nsn_function(|number| self.format_nsn(number, metadata, number_format))
                .with_ext(Self::get_formatted_extension(
                    phone_number,
                    metadata,
                    number_format,
                ));
            return Ok(formatted_number_builder.build()?.into());
        }
        Ok(formatted_number_builder.early_exit().into())
    }

    /// Returns the region code that matches the specific country calling code. In
    /// the case of no region code being found, the unknown region code will be
    /// returned.
    /// # Arguments
    ///
    /// * `country_calling_code` - The country calling code.
    pub(crate) fn get_region_code_for_country_code(
        &self,
        country_calling_code: i32,
    ) -> Option<&str> {
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        region_codes.and_then(|mut codes| codes.next())
    }

    /// Returns the region codes that matches the specific country calling code. In
    /// the case of no region code being found, region_codes will be left empty.
    ///
    /// # Arguments
    ///
    /// * `country_calling_code` - The country calling code.
    pub(crate) fn get_region_codes_for_country_calling_code(
        &self,
        country_calling_code: i32,
    ) -> Option<impl ExactSizeIterator<Item = &str>> {
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

    fn get_metadata_for_calling_code(
        &self,
        country_calling_code: i32,
    ) -> Option<&PhoneMetadataWrapper> {
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        region_code.and_then(|region_code| {
            self.get_metadata_for_region_or_calling_code(country_calling_code, region_code)
        })
    }

    pub(crate) fn get_metadata_for_region_or_calling_code(
        &self,
        country_calling_code: i32,
        region_code: &str,
    ) -> Option<&PhoneMetadataWrapper> {
        if REGION_CODE_FOR_NON_GEO_ENTITY == region_code {
            self.country_code_to_non_geographical_metadata_map
                .get(&country_calling_code)
        } else {
            self.region_to_metadata_map.get(region_code)
        }
    }

    pub(crate) fn format_nsn<'b>(
        &self,
        phone_number: &'b str,
        metadata: &PhoneMetadataWrapper,
        number_format: PhoneNumberFormat,
    ) -> RegexResult<Cow<'b, str>> {
        self.format_nsn_with_carrier(phone_number, metadata, number_format, "")
    }

    pub(crate) fn format_nsn_with_carrier<'b>(
        &self,
        number: &'b str,
        metadata: &PhoneMetadataWrapper,
        number_format: PhoneNumberFormat,
        carrier_code: &str,
    ) -> RegexResult<Cow<'b, str>> {
        // When the intl_number_formats exists, we use that to format national number
        // for the INTERNATIONAL format instead of using the number_formats.
        let available_formats = if metadata.intl_number_format.is_empty()
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

    pub(crate) fn choose_formatting_pattern_for_number<'b>(
        &self,
        available_formats: &'b [NumberFormatWrapper],
        national_number: &str,
    ) -> RegexResult<Option<&'b NumberFormatWrapper>> {
        for format in available_formats {
            if !format
                .leading_digits_pattern()
                // We always use the last leading_digits_pattern, as it is the most
                // detailed.
                .last()
                .map(|last| {
                    last.anchor_start()
                        .map(|last| last.is_some_and(|p| p.is_match(national_number)))
                })
                // default not continue
                .unwrap_or(Ok(true))?
            {
                continue;
            }
            let pattern_to_match = format.pattern();

            if pattern_to_match
                .anchor_full()?
                .is_some_and(|p| p.is_match(national_number))
            {
                return Ok(Some(format));
            }
        }
        Ok(None)
    }

    // Note that carrier_code is optional - if an empty string, no carrier code
    // replacement will take place.
    pub(crate) fn format_nsn_using_pattern_with_carrier<'b>(
        &self,
        national_number: &'b str,
        formatting_pattern: &NumberFormatWrapper,
        number_format: PhoneNumberFormat,
        carrier_code: &str,
    ) -> RegexResult<Cow<'b, str>> {
        let mut number_format_rule = Cow::Borrowed(&formatting_pattern.original.format);
        if matches!(number_format, PhoneNumberFormat::National)
            && !carrier_code.is_empty()
            && !formatting_pattern
                .original
                .domestic_carrier_code_formatting_rule()
                .is_empty()
        {
            // Replace the $CC in the formatting rule with the desired carrier code.
            let mut carrier_code_formatting_rule = Cow::Borrowed(
                formatting_pattern
                    .original
                    .domestic_carrier_code_formatting_rule(),
            );

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
            let national_prefix_formatting_rule = formatting_pattern
                .original
                .national_prefix_formatting_rule();

            if matches!(number_format, PhoneNumberFormat::National)
                && !national_prefix_formatting_rule.is_empty()
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

        let pattern_to_match = formatting_pattern.pattern();

        let mut formatted_number = pattern_to_match
            .original()?
            .map(|p| p.replace_all(national_number, number_format_rule.as_str()))
            .unwrap_or(national_number.into());

        if matches!(number_format, PhoneNumberFormat::RFC3966) {
            // First consume any leading punctuation, if any was present.
            if let Some(matches) = self
                .reg_exps
                .separator_pattern_anchor_start
                .find(&formatted_number)
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
    pub(crate) fn format_nsn_using_pattern<'b>(
        &self,
        national_number: &'b str,
        formatting_pattern: &NumberFormatWrapper,
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
    pub(crate) fn get_formatted_extension<'a, 'b>(
        phone_number: &'a PhoneNumber,
        metadata: &'b PhoneMetadataWrapper,
        number_format: PhoneNumberFormat,
    ) -> Option<(&'b str, &'a str)> {
        if phone_number.extension.is_none() || phone_number.extension().is_empty() {
            return None;
        }

        let prefix = if matches!(number_format, PhoneNumberFormat::RFC3966) {
            RFC3966_EXTN_PREFIX
        } else if metadata.original.preferred_extn_prefix.is_some() {
            metadata.original.preferred_extn_prefix()
        } else {
            DEFAULT_EXTN_PREFIX
        };
        Some((prefix, phone_number.extension()))
    }

    /// Formats a phone number using a user-defined pattern.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `number_format` - The phone number format to apply.
    /// * `user_defined_formats` - A slice of user-defined formatting patterns.
    pub(crate) fn format_by_pattern(
        &self,
        phone_number: &PhoneNumber,
        number_format: PhoneNumberFormat,
        user_defined_formats: &[NumberFormatWrapper],
    ) -> RegexResult<String> {
        let country_calling_code = phone_number.country_code;
        // Note GetRegionCodeForCountryCode() is used because formatting information
        // contained in the metadata for US.
        let formatted_number_builder =
            new_formatted_number_builder(phone_number, Some(number_format));
        let Some(metadata) = self.get_metadata_for_calling_code(country_calling_code) else {
            return Ok(formatted_number_builder.early_exit());
        };

        formatted_number_builder
            .with_format_nsn_function(|national_significant_number| {
                let formatting_pattern = self.choose_formatting_pattern_for_number(
                    user_defined_formats,
                    national_significant_number,
                )?;

                if let Some(formatting_pattern) = formatting_pattern {
                    // Before we do a replacement of the national prefix pattern $NP with the
                    // national prefix, we need to copy the rule so that subsequent replacements
                    // for different numbers have the appropriate national prefix.
                    let mut num_format_copy = formatting_pattern.clone();

                    let national_prefix_formatting_rule = formatting_pattern
                        .original
                        .national_prefix_formatting_rule();
                    if !national_prefix_formatting_rule.is_empty() {
                        let national_prefix = metadata.original.national_prefix();
                        if !national_prefix.is_empty() {
                            // Replace $NP with national prefix and $FG with the first group ($1).
                            let rule = national_prefix_formatting_rule
                                .replace("$NP", national_prefix)
                                .replace("$FG", "$1");
                            num_format_copy.original.national_prefix_formatting_rule = Some(rule);
                        } else {
                            // We don't want to have a rule for how to format the national prefix if
                            // there isn't one.
                            num_format_copy.original.national_prefix_formatting_rule = None;
                        }
                    }
                    Ok(self.format_nsn_using_pattern(
                        national_significant_number,
                        &num_format_copy,
                        number_format,
                    )?)
                } else {
                    Ok(national_significant_number.into())
                }
            })
            .with_ext(Self::get_formatted_extension(
                phone_number,
                metadata,
                PhoneNumberFormat::National,
            ))
            .build()
    }

    /// Formats a national number with a specific carrier code.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `carrier_code` - The carrier code to include in the formatted number.
    pub(crate) fn format_national_number_with_carrier_code(
        &self,
        phone_number: &PhoneNumber,
        carrier_code: &str,
    ) -> RegexResult<String> {
        let country_calling_code = phone_number.country_code;
        let formatted_number_builder =
            new_formatted_number_builder(phone_number, Some(PhoneNumberFormat::National));

        // Note GetRegionCodeForCountryCode() is used because formatting information
        // contained in the metadata for US.
        let Some(metadata) = self.get_metadata_for_calling_code(country_calling_code) else {
            return Ok(formatted_number_builder.early_exit());
        };

        let formatted_number = formatted_number_builder
            .with_format_nsn_function(|national_significant_number| {
                self.format_nsn_with_carrier(
                    national_significant_number,
                    metadata,
                    PhoneNumberFormat::National,
                    carrier_code,
                )
            })
            .with_ext(Self::get_formatted_extension(
                phone_number,
                metadata,
                PhoneNumberFormat::National,
            ))
            .build()?;

        Ok(formatted_number)
    }

    /// Formats a national number, inserting the preferred domestic carrier code if available.
    /// Otherwise, uses the provided fallback carrier code.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `fallback_carrier_code` - The carrier code to use if a preferred one is not available.
    pub(crate) fn format_national_number_with_preferred_carrier_code(
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

    /// Formats a phone number for dialing from a mobile device in a specific region.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `calling_from` - The region where the call is being placed.
    /// * `with_formatting` - Whether to include formatting characters.
    pub(crate) fn format_number_for_mobile_dialing<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        calling_from: &str,
        with_formatting: bool,
    ) -> RegexResult<Option<Cow<'b, str>>> {
        let country_calling_code = phone_number.country_code;

        // Get metadata marking region as valid
        let Some(metadata) = self.get_metadata_for_calling_code(country_calling_code) else {
            return if phone_number.raw_input.is_some() {
                Ok(Some(phone_number.raw_input().into()))
            } else {
                Ok(None)
            };
        };

        let formatted_number_builder = new_formatted_number_builder(phone_number, None);
        // Clear the extension, as that part cannot normally be dialed together with
        // the main number.
        let mut number_no_extension = phone_number.clone();
        number_no_extension.extension = None;
        let region_code = self.get_region_code_for_country_code(country_calling_code);
        let number_type = self.get_number_type(&number_no_extension)?;
        let is_valid_number = !matches!(number_type, PhoneNumberType::Unknown);
        let formatted_number = if let Some(region_code) = region_code
            && calling_from == region_code
        {
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
                    Some(
                        formatted_number_builder
                            .with_format_nsn_function(|_| {
                                Ok(self
                                    .format_national_number_with_preferred_carrier_code(
                                        &number_no_extension,
                                        "",
                                    )?
                                    .into())
                            })
                            .build()?,
                    )
                } else {
                    // Brazilian fixed line and mobile numbers need to be dialed with a
                    // carrier code when called within Brazil. Without that, most of the
                    // carriers won't connect the call. Because of that, we return an empty
                    // string here.
                    None
                }
            } else if country_calling_code == NANPA_COUNTRY_CODE {
                // For NANPA countries, we output international format for numbers that
                // can be dialed internationally, since that always works, except for
                // numbers which might potentially be short numbers, which are always
                // dialled in national format.
                Some(
                    formatted_number_builder
                        .with_format_nsn_function(|national_number| {
                            let format = if self
                                .can_be_internationally_dialled(&number_no_extension)?
                                && !test_number_length_with_unknown_type(national_number, metadata)
                                    .is_err_and(|e| matches!(e, ValidationError::TooShort))
                            {
                                PhoneNumberFormat::International
                            } else {
                                PhoneNumberFormat::National
                            };
                            Ok(self
                                .format(&number_no_extension, format)?
                                .to_string()
                                .into())
                        })
                        .build()?,
                )
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
                Some(
                    formatted_number_builder
                        .with_format_nsn_function(|_| {
                            Ok(self
                                .format(&number_no_extension, format)?
                                .to_string()
                                .into())
                        })
                        .build()?,
                )
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
            return Ok(Some(
                formatted_number_builder
                    .with_format_nsn_function(|_| {
                        Ok(self
                            .format(&number_no_extension, format)?
                            .to_string()
                            .into())
                    })
                    .build()?
                    .into(),
            ));
        } else {
            None
        };
        let Some(formatted_number) = formatted_number else {
            return Ok(None);
        };
        if !with_formatting {
            Ok(Some(Cow::Owned(
                self.normalize_diallable_chars_only(&formatted_number),
            )))
        } else {
            Ok(Some(Cow::Owned(formatted_number)))
        }
    }

    /// Gets the type of a phone number (e.g., FIXED_LINE, MOBILE, TOLL_FREE).
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to get the type for.
    pub(crate) fn get_number_type(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<PhoneNumberType> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) = region_code.and_then(|region_code| {
            self.get_metadata_for_region_or_calling_code(phone_number.country_code, region_code)
        }) else {
            return Ok(PhoneNumberType::Unknown);
        };
        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_significant_number = get_national_significant_number(phone_number, &mut buf);
        self.get_number_type_helper(&national_significant_number, metadata)
    }

    /// Gets the region code for a given phone number.
    /// Returns None if the number is invalid or does not belong to a specific region.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to get the region for.
    pub(crate) fn get_region_code_for_number(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<Option<&str>> {
        let country_calling_code: i32 = phone_number.country_code;

        let default = || {
            trace!(
                "Missing/invalid country calling code ({})",
                country_calling_code
            );
            None
        };
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        let Some(mut region_codes) = region_codes else {
            return Ok(default());
        };
        let count = region_codes.len();
        if count > 1 {
            return Ok(self
                .get_region_code_for_number_from_region_list(phone_number, region_codes)?
                .or_else(default));
        }

        Ok(region_codes.next().or_else(default))
    }

    pub(crate) fn get_region_code_for_number_from_region_list<'b>(
        &self,
        phone_number: &PhoneNumber,
        region_codes: impl Iterator<Item = &'b str>,
    ) -> RegexResult<Option<&'b str>> {
        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_number = get_national_significant_number(phone_number, &mut buf);
        for code in region_codes {
            // Metadata cannot be NULL because the region codes come from the country
            // calling code map.
            let Some(metadata) = &self.region_to_metadata_map.get(code) else {
                return Ok(None);
            };
            if metadata
                .leading_digits()
                .anchor_start()?
                .is_some_and(|a| a.is_match(&national_number))
                || self.get_number_type_helper(&national_number, metadata)?
                    != PhoneNumberType::Unknown
            {
                return Ok(Some(code));
            }
        }
        Ok(None)
    }

    pub(crate) fn get_number_type_helper(
        &self,
        national_number: &str,
        metadata: &PhoneMetadataWrapper,
    ) -> RegexResult<PhoneNumberType> {
        if !self.is_number_matching_desc(national_number, &metadata.general_desc)? {
            trace!(
                "Number '{national_number}' type unknown - doesn't match general national number pattern"
            );
            return Ok(PhoneNumberType::Unknown);
        }
        macro_rules! match_desc {
            ($($pat_name:ident => $enum:expr; $desc:literal),*) => {
                $(
                if self.is_number_matching_desc(national_number, &metadata.$pat_name)? {
                    trace!(
                        concat!("Number '{}' is a ", $desc, " number.",),
                        national_number
                    );
                    return Ok($enum);
                }
                )*
            };
        }

        match_desc!(
            premium_rate    => PhoneNumberType::PremiumRate;    "premium",
            toll_free       => PhoneNumberType::TollFree;       "toll-free",
            shared_cost     => PhoneNumberType::SharedCost;     "shared cost",
            voip            => PhoneNumberType::VoIP;           "VOIP (Voice over IP)",
            personal_number => PhoneNumberType::PersonalNumber; "personal number",
            pager           => PhoneNumberType::Pager;          "pager number",
            uan             => PhoneNumberType::UAN;            "UAN",
            voicemail       => PhoneNumberType::VoiceMail;      "voicemail number"
        );

        let is_fixed_line = self.is_number_matching_desc(national_number, &metadata.fixed_line)?;
        if is_fixed_line {
            if metadata.original.same_mobile_and_fixed_line_pattern() {
                trace!(
                    "Number '{national_number}': fixed-line and mobile patterns equal,\
                 number is fixed-line or mobile"
                );
                return Ok(PhoneNumberType::FixedLineOrMobile);
            } else if self.is_number_matching_desc(national_number, &metadata.mobile)? {
                trace!(
                    "Number '{national_number}': Fixed-line and mobile patterns differ, but number is \
                        still fixed-line or mobile"
                );
                return Ok(PhoneNumberType::FixedLineOrMobile);
            }
            trace!("Number '{national_number}' is a fixed line number.");
            return Ok(PhoneNumberType::FixedLine);
        }
        // Otherwise, test to see if the number is mobile. Only do this if certain
        // that the patterns for mobile and fixed line aren't the same.
        if !metadata.original.same_mobile_and_fixed_line_pattern()
            && self.is_number_matching_desc(national_number, &metadata.mobile)?
        {
            trace!("Number '{national_number}' is a mobile number.");
            return Ok(PhoneNumberType::Mobile);
        }
        trace!(
            "Number'{national_number}' type unknown - doesn\'t match any specific number type pattern."
        );
        Ok(PhoneNumberType::Unknown)
    }

    pub(crate) fn is_number_matching_desc(
        &self,
        national_number: &str,
        number_desc: &PhoneNumberDescWrapper,
    ) -> RegexResult<bool> {
        // Check if any possible number lengths are present; if so, we use them to
        // avoid checking the validation pattern if they don't match. If they are
        // absent, this means they match the general description, which we have
        // already checked before checking a specific number type.
        let actual_length = national_number.len() as i32;
        if !number_desc.original.possible_length.is_empty()
            && !number_desc
                .original
                .possible_length
                .contains(&actual_length)
        {
            return Ok(false);
        }
        // very common name, so specify mod
        Ok(helper_functions::is_match(
            &self.matcher_api,
            national_number,
            number_desc,
        )?)
    }

    /// Checks if a number can be dialled internationally.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    pub(crate) fn can_be_internationally_dialled(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<bool> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) =
            region_code.and_then(|region_code| self.region_to_metadata_map.get(region_code))
        else {
            // Note numbers belonging to non-geographical entities (e.g. +800 numbers)
            // are always internationally diallable, and will be caught here.
            return Ok(true);
        };

        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_significant_number = get_national_significant_number(phone_number, &mut buf);
        Ok(!self.is_number_matching_desc(
            &national_significant_number,
            &metadata.no_international_dialling,
        )?)
    }

    pub(crate) fn normalize_diallable_chars_only(&self, phone_number: &str) -> String {
        normalize_helper(&self.reg_exps.diallable_char_mappings, true, phone_number)
    }

    /// Normalizes a string of characters representing a phone number.
    /// This performs the following mappings: replaces alpha characters with digits removes all other punctuation and formatting characters.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number string to normalize.
    pub(crate) fn normalize_digits_only(&self, phone_number: &str) -> String {
        phone_number
            .chars()
            .filter_map(|c| {
                uniprops_digits::uniprops::get_digit_value(c).map(|d| (d + b'0') as char)
            })
            .collect()
    }

    /// Formats a phone number for calling from outside the number's region.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `calling_from` - The region where the call is being placed from.
    pub(crate) fn format_out_of_country_calling_number<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        calling_from: &str,
    ) -> RegexResult<Cow<'a, str>> {
        let Some(metadata_calling_from) = self.region_to_metadata_map.get(calling_from) else {
            trace!(
                "Trying to format number from invalid region {calling_from}\
              . International formatting applied."
            );
            return self.format(phone_number, PhoneNumberFormat::International);
        };
        let country_code = phone_number.country_code;
        let formatted_number_builder = new_formatted_number_builder(phone_number, None);
        let Some(metadata_for_region) = self.get_metadata_for_calling_code(country_code) else {
            return Ok(formatted_number_builder.early_exit().into());
        };
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
        } else if country_code == metadata_calling_from.original.country_code() {
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
            return self.format(phone_number, PhoneNumberFormat::National);
        }
        // Metadata cannot be NULL because we checked 'IsValidRegionCode()' above.
        let international_prefix = metadata_calling_from.international_prefix().original_base();

        // In general, if there is a preferred international prefix, use that.
        // international format of the number is returned since we would not know
        // which one to use.
        let international_prefix_for_formatting = if metadata_calling_from
            .original
            .preferred_international_prefix
            .is_some()
        {
            Some(
                metadata_calling_from
                    .original
                    .preferred_international_prefix(),
            )
        } else if self
            .reg_exps
            .single_international_prefix_fullmatch
            .is_match(international_prefix)
        {
            Some(international_prefix)
        } else {
            None
        };

        let formatted_number = formatted_number_builder
            .with_format_nsn_function(|national_significant_number| {
                self.format_nsn(
                    national_significant_number,
                    metadata_for_region,
                    PhoneNumberFormat::International,
                )
            })
            .with_ext(Self::get_formatted_extension(
                phone_number,
                metadata_for_region,
                PhoneNumberFormat::International,
            ))
            .with_get_prefix_function(|country_code| {
                if let Some(international_prefix_for_formatting) =
                    international_prefix_for_formatting
                {
                    PrefixParts::Parts4([
                        international_prefix_for_formatting.to_string().into(),
                        " ".into(),
                        country_code.into(),
                        " ".into(),
                    ])
                } else {
                    get_number_prefix_by_format_and_calling_code(
                        country_code,
                        PhoneNumberFormat::International,
                    )
                }
            })
            .build()?;

        Ok(formatted_number.into())
    }

    pub(crate) fn has_formatting_pattern_for_number(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<bool> {
        let country_calling_code = phone_number.country_code;
        let Some(metadata) = self.get_metadata_for_calling_code(country_calling_code) else {
            return Ok(false);
        };
        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_number = get_national_significant_number(phone_number, &mut buf);
        let format_rule =
            self.choose_formatting_pattern_for_number(&metadata.number_format, &national_number);
        format_rule.map(|rule| rule.is_some())
    }

    /// Formats a phone number in the original format that it was parsed from.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `region_calling_from` - The region from which the number was originally parsed.
    pub(crate) fn format_in_original_format<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        region_calling_from: &str,
    ) -> RegexResult<Cow<'a, str>> {
        if phone_number.raw_input.is_some()
            && !self.has_formatting_pattern_for_number(phone_number)?
        {
            // We check if we have the formatting pattern because without that, we might
            // format the number as a group without national prefix.
            return Ok(Cow::Borrowed(phone_number.raw_input()));
        }
        if phone_number.country_code_source.is_none() {
            return self.format(phone_number, PhoneNumberFormat::National);
        }
        let formatted_number = match phone_number.country_code_source() {
            CountryCodeSource::FromNumberWithPlusSign => {
                self.format(phone_number, PhoneNumberFormat::International)?
            }
            CountryCodeSource::FromNumberWithIdd => {
                self.format_out_of_country_calling_number(phone_number, region_calling_from)?
            }
            CountryCodeSource::FromNumberWithoutPlusSign => Cow::Owned(
                self.format(phone_number, PhoneNumberFormat::International)?[1..].to_string(),
            ),
            CountryCodeSource::FromDefaultCountry
            | CountryCodeSource::Unspecified => 'default_block: {
                let format_national = || self.format(phone_number, PhoneNumberFormat::National);

                let region_code = self.get_region_code_for_country_code(phone_number.country_code);
                // We strip non-digits from the NDD here, and from the raw input later, so
                // that we can compare them easily.
                let Some(national_prefix) = region_code.and_then(|region_code| {
                    self.get_ndd_prefix_for_region(region_code, true /* strip non-digits */)
                }) else {
                    break 'default_block format_national()?;
                };
                let Some(metadata) = region_code
                    .and_then(|region_code| self.region_to_metadata_map.get(region_code))
                else {
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
                // TODO: move complex logic to builder
                let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
                let national_number = get_national_significant_number(phone_number, &mut buf);
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
                let candidate_national_prefix_rule =
                    format_rule.original.national_prefix_formatting_rule();
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
                        self.normalize_digits_only(candidate_national_prefix_rule)
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
                number_format.original.national_prefix_formatting_rule = None;
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
        Ok(formatted_number.to_string().into())
    }

    /// Check if raw_input, which is assumed to be in the national format, has a
    /// national prefix. The national prefix is assumed to be in digits-only form.
    pub(crate) fn raw_input_contains_national_prefix(
        &self,
        raw_input: &str,
        national_prefix: &str,
        region_code: Option<&str>,
    ) -> RegexResult<bool> {
        let normalized_national_number = self.normalize_digits_only(raw_input);
        if let Some(stripped) = normalized_national_number.strip_prefix(national_prefix) {
            // Some Japanese numbers (e.g. 00777123) might be mistaken to contain
            // the national prefix when written without it (e.g. 0777123) if we just
            // do prefix matching. To tackle that, we check the validity of the
            // number if the assumed national prefix is removed (777123 won't be
            // valid in Japan).
            if let Ok(number_without_national_prefix) = self.parse(stripped, region_code) {
                return self.is_valid_number(&number_without_national_prefix);
            }
        }
        Ok(false)
    }

    /// Parses a string into a phone number object.
    ///
    /// # Arguments
    ///
    /// * `number_to_parse` - The number string to parse.
    /// * `default_region` - The region to assume if the number is not in international format.
    pub(crate) fn parse(
        &self,
        number_to_parse: &str,
        default_region: Option<&str>,
    ) -> ParseResult<PhoneNumber> {
        self.parse_helper(number_to_parse, default_region, false, true)
    }

    /// Parses a string into a phone number object, keeping the raw input.
    ///
    /// # Arguments
    ///
    /// * `number_to_parse` - The number string to parse.
    /// * `default_region` - The region to assume if the number is not in international format.
    pub(crate) fn parse_and_keep_raw_input(
        &self,
        number_to_parse: &str,
        default_region: Option<&str>,
    ) -> ParseResult<PhoneNumber> {
        self.parse_helper(number_to_parse, default_region, true, true)
    }

    /// Checks if a phone number is valid.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to validate.
    pub(crate) fn is_valid_number(&self, phone_number: &PhoneNumber) -> RegexResult<bool> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        if let Some(region_code) = region_code {
            self.is_valid_number_for_region(phone_number, region_code)
        } else {
            Ok(false)
        }
    }

    /// Checks if a phone number is valid for a specific region.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to validate.
    /// * `region_code` - The region to validate against.
    pub(crate) fn is_valid_number_for_region(
        &self,
        phone_number: &PhoneNumber,
        region_code: &str,
    ) -> RegexResult<bool> {
        let country_code = phone_number.country_code;
        let metadata = self.get_metadata_for_region_or_calling_code(country_code, region_code);
        if let Some(metadata) = metadata.filter(|metadata| {
            !(REGION_CODE_FOR_NON_GEO_ENTITY != region_code
                && country_code != metadata.original.country_code())
        }) {
            let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
            let national_number = get_national_significant_number(phone_number, &mut buf);
            Ok(!matches!(
                self.get_number_type_helper(&national_number, metadata)?,
                PhoneNumberType::Unknown
            ))
        } else {
            Ok(false)
        }
    }

    /// Formats a phone number for out-of-country dialing, preserving any alphabetic characters.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to format.
    /// * `calling_from` - The region where the call is being placed from.
    pub(crate) fn format_out_of_country_keeping_alpha_chars<'a>(
        &self,
        phone_number: &'a PhoneNumber,
        calling_from: &str,
    ) -> RegexResult<Cow<'a, str>> {
        if phone_number.raw_input().is_empty() {
            return Ok(self
                .format_out_of_country_calling_number(phone_number, calling_from)?
                .to_string()
                .into());
        }

        let country_code = phone_number.country_code;
        let Some(metadata_for_region) = self.get_metadata_for_calling_code(country_code) else {
            return Ok(phone_number.raw_input().into());
        };

        let mut normalized_raw_input = helper_functions::normalize_helper(
            &self.reg_exps.all_plus_number_grouping_symbols,
            true,
            phone_number.raw_input(),
        );

        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_number = get_national_significant_number(phone_number, &mut buf);

        if national_number.len() > 3
            && let Some(first_national_number_digit) =
                normalized_raw_input.find(&national_number[0..3])
        {
            normalized_raw_input.drain(0..first_national_number_digit);
        }

        // Удалено дублирование: оставили только один блок для NANPA
        if country_code == NANPA_COUNTRY_CODE && self.nanpa_regions.contains(calling_from) {
            let mut buf = itoa::Buffer::new();
            return Ok(
                fast_cat::concat_str!(buf.format(country_code), " ", &normalized_raw_input).into(),
            );
        }

        let metadata = self.region_to_metadata_map.get(calling_from);

        if let Some(metadata) =
            metadata.filter(|metadata| country_code == metadata.original.country_code())
        {
            let Some(formatting_pattern) = self
                .choose_formatting_pattern_for_number(&metadata.number_format, &national_number)?
            else {
                return Ok(normalized_raw_input.into());
            };

            let mut new_format = formatting_pattern.clone();
            // TODO: optimize with more consistent logic
            // We can ignore error since we just created lock
            new_format.set_pattern(self.reg_exps.catch_all_formatting_regex.clone());

            new_format.original.format = "$1$2".to_owned();

            return self
                .format_nsn_using_pattern(
                    &normalized_raw_input,
                    &new_format,
                    PhoneNumberFormat::National,
                )
                .map(|cow| Cow::Owned(cow.into_owned()));
        }

        let international_prefix_for_formatting = metadata.map(|metadata| {
            let international_prefix = metadata.international_prefix().original_base();
            if self
                .reg_exps
                .single_international_prefix_fullmatch
                .is_match(international_prefix)
            {
                international_prefix
            } else {
                metadata.original.preferred_international_prefix()
            }
        });

        let result = new_formatted_number_builder(phone_number, None)
            .with_format_nsn_function(|_| {
                let (number_no_prefix, _) = self.maybe_strip_extension(&normalized_raw_input);
                // Аллокация здесь обязательна, т.к. normalized_raw_input уничтожается в конце функции
                Ok(number_no_prefix.to_string().into())
            })
            // Переименован аргумент, чтобы не было затенения (shadowing)
            .with_get_prefix_function(|formatted_country_code| {
                if let Some(international_prefix) = international_prefix_for_formatting {
                    PrefixParts::Parts4([
                        // TODO: remove alloc
                        international_prefix.to_string().into(),
                        " ".into(),
                        formatted_country_code.into(),
                        " ".into()
                    ])
                } else {
                    if !self.region_to_metadata_map.contains_key(calling_from) {
                        trace!(
                            "Trying to format number from invalid region {}. International formatting applied.",
                            calling_from
                        );
                    }
                    get_number_prefix_by_format_and_calling_code(
                        formatted_country_code,
                        PhoneNumberFormat::International,
                    )
                }
            })
            .with_ext(Self::get_formatted_extension(
                phone_number,
                metadata_for_region,
                PhoneNumberFormat::International,
            ))
            .build()?
            .into();

        Ok(result)
    }

    /// Returns whether the value of phoneContext follows the syntax defined in
    /// RFC3966.
    pub(crate) fn is_phone_context_valid(&self, phone_context: &str) -> bool {
        if phone_context.is_empty() {
            return false;
        }

        // Does phone-context value match pattern of global-number-digits or
        // domainname
        self.reg_exps
            .rfc3966_global_number_digits_pattern_fullmatch
            .is_match(phone_context)
            || self
                .reg_exps
                .rfc3966_domainname_pattern_fullmatch
                .is_match(phone_context)
    }

    /// Converts number_to_parse to a form that we can parse and write it to
    /// national_number if it is written in RFC3966; otherwise extract a possible
    /// number out of it and write to national_number.
    pub(crate) fn build_national_number_for_parsing(
        &self,
        number_to_parse: &str,
    ) -> ParseResult<String> {
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
            national_number.push_str(
                self.extract_possible_number(number_to_parse)
                    .map_err(|err| InternalError::from(err).translate::<ParseError>())?,
            );
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
        Ok(national_number)
    }

    /// Extracts the value of the phone-context parameter of number_to_extract_from
    /// where the index of ";phone-context=" is parameter index_of_phone_context,
    /// following the syntax defined in RFC3966.
    ///
    /// Returns the extracted `Some(possibly empty)`, or a `None` if no
    /// phone-context parameter is found.
    pub(crate) fn extract_phone_context(
        number_to_extract_from: &str,
        index_of_phone_context: usize,
    ) -> &str {
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
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The string to extract the number from.
    pub(crate) fn extract_possible_number<'a>(
        &self,
        phone_number: &'a str,
    ) -> ExtractNumberResult<&'a str> {
        // Rust note: skip UTF-8 validation since in rust strings are already UTF-8 valid

        // inline regexp search
        let Some(i) = phone_number.find(|c: char| {
            uniprops_digits::uniprops::get_digit_value(c).is_some() || PLUS_CHARS.contains(c)
        }) else {
            // No valid start character was found. extracted_number should be set to
            // empty string.
            return Err(ExtractNumberError::NoValidStartCharacter);
        };

        let mut extracted_number = &phone_number[i..];
        extracted_number = self.trim_unwanted_end_chars(extracted_number);
        if extracted_number.is_empty() {
            return Err(ExtractNumberError::NotANumber);
        }

        // Now remove any extra numbers at the end.
        Ok(self
            .reg_exps
            .capture_up_to_second_number_start_pattern
            .captures(extracted_number)
            .and_then(|c| c.get(1))
            .map(move |m| m.as_str())
            .unwrap_or(extracted_number))
    }

    /// Checks if a phone number is a possible number.
    /// This is a less strict check than `is_valid_number`.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    pub(crate) fn is_possible_number(&self, phone_number: &PhoneNumber) -> bool {
        self.is_possible_number_with_reason(phone_number).is_ok()
    }

    /// Checks if a phone number is a possible number of a specific type.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    /// * `phone_number_type` - The type of number to check for.
    pub(crate) fn is_possible_number_for_type(
        &self,
        phone_number: &PhoneNumber,
        phone_number_type: PhoneNumberType,
    ) -> bool {
        self.is_possible_number_for_type_with_reason(phone_number, phone_number_type)
            .is_ok()
    }

    /// Checks if a string could be a possible phone number for a given region.
    ///
    /// # Arguments
    ///
    /// * `number` - The number string to check.
    /// * `region_code` - The region to check against.
    pub(crate) fn is_possible_number_for_string(
        &self,
        phone_number: &str,
        region_dialing_from: &str,
    ) -> bool {
        match self.parse(phone_number, Some(region_dialing_from)) {
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

    pub(crate) fn is_possible_number_with_reason(
        &self,
        phone_number: &PhoneNumber,
    ) -> ValidationResult {
        self.is_possible_number_for_type_with_reason(phone_number, PhoneNumberType::Unknown)
    }

    pub(crate) fn is_possible_number_for_type_with_reason(
        &self,
        phone_number: &PhoneNumber,
        phone_number_type: PhoneNumberType,
    ) -> ValidationResult {
        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let national_number: Cow<'_, str> = get_national_significant_number(phone_number, &mut buf);
        let country_code = phone_number.country_code;
        // Note: For regions that share a country calling code, like NANPA numbers, we
        // just use the rules from the default region (US in this case) since the
        // GetRegionCodeForNumber will not work if the number is possible but not
        // valid. There is in fact one country calling code (290) where the possible
        // number pattern differs between various regions (Saint Helena and Tristan da
        // Cuñha), but this is handled by putting all possible lengths for any country
        // with this country calling code in the metadata for the default region in
        // this case.
        let Some(metadata) = self.get_metadata_for_calling_code(country_code) else {
            return Err(ValidationError::InvalidCountryCode);
        };
        test_number_length(&national_number, metadata, phone_number_type)
    }

    /// Truncates number untill it's valid
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The number to truncate
    pub(crate) fn truncate_too_long_number(
        &self,
        phone_number: &mut PhoneNumber,
    ) -> RegexResult<bool> {
        if self.is_valid_number(phone_number)? {
            return Ok(true);
        }
        let mut number_copy = phone_number.clone();
        let mut national_number = phone_number.national_number;
        loop {
            national_number /= 10;
            number_copy.national_number = national_number;
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
        phone_number.national_number = national_number;
        Ok(true)
    }

    // Note if any new field is added to this method that should always be filled
    // in, even when keepRawInput is false, it should also be handled in the
    // CopyCoreFieldsOnly() method.
    pub(crate) fn parse_helper(
        &self,
        number_to_parse: &str,
        default_region: Option<&str>,
        keep_raw_input: bool,
        check_region: bool,
    ) -> ParseResult<PhoneNumber> {
        let national_number = self.build_national_number_for_parsing(number_to_parse)?;
        if !self.is_viable_phone_number(&national_number) {
            trace!("The string supplied did not seem to be a phone number '{national_number}'.");
            return Err(
                ParseError::NotANumber(NotANumberError::NotMatchedValidNumberPattern).into(),
            );
        }

        if let Some(default_region) = default_region
            && check_region
            && !self.check_region_for_parsing(&national_number, default_region)
        {
            trace!("Missing or invalid default country.");
            return Err(ParseError::InvalidCountryCode.into());
        }
        let mut temp_number = PhoneNumber::default();
        if keep_raw_input {
            temp_number.raw_input = Some(number_to_parse.to_owned());
        }
        // Attempt to parse extension first, since it doesn't require country-specific
        // data and we want to have the non-normalised number here.

        let (national_number, extension) = self.maybe_strip_extension(&national_number);

        if let Some(extension) = extension {
            temp_number.extension = Some(extension.to_owned());
        }
        let mut country_metadata = default_region.and_then(|m| self.get_metadata_for_region(m));
        // Check to see if the number is given in international format so we know
        // whether this number is from the default country or not.
        let mut normalized_national_number = self
            .maybe_extract_country_code(
                country_metadata,
                keep_raw_input,
                national_number,
                &mut temp_number,
            )
            .or_else(|err| {
                if !matches!(err, InternalError::Wrapped(ParseError::InvalidCountryCode)) {
                    return Err(err);
                }
                let plus_match = self.reg_exps.plus_chars_pattern_start.find(national_number);
                if let Some(plus_match) = plus_match {
                    let normalized_national_number = &national_number[plus_match.end()..];
                    // Strip the plus-char, and try again.
                    let normalized_national_number = self.maybe_extract_country_code(
                        country_metadata,
                        keep_raw_input,
                        normalized_national_number,
                        &mut temp_number,
                    )?;
                    if temp_number.country_code == 0 {
                        return Err(ParseError::InvalidCountryCode.into());
                    }
                    return Ok(normalized_national_number);
                }
                Err(err)
            })?;

        let mut country_code = temp_number.country_code;
        if country_code != 0 {
            let phone_number_region = self.get_region_code_for_country_code(country_code);
            if phone_number_region != default_region {
                country_metadata = phone_number_region.and_then(|phone_number_region| {
                    self.get_metadata_for_region_or_calling_code(country_code, phone_number_region)
                });
            }
        } else if let Some(country_metadata) = country_metadata {
            // If no extracted country calling code, use the region supplied instead.
            // Note that the national number was already normalized by
            // MaybeExtractCountryCode.
            country_code = country_metadata.original.country_code();
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

            let (phone_number, carrier_code) = self
                .maybe_strip_national_prefix_and_carrier_code(
                    country_metadata,
                    &potential_national_number,
                )
                .map_err(|err| err.translate_internal())?;

            let carrier_code = carrier_code.map(|c| c.to_string());

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
                    temp_number.preferred_domestic_carrier_code = Some(carrier_code.to_owned());
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
        temp_number.country_code = country_code;

        if let Some(zeroes_count) =
            Self::get_italian_leading_zeros_for_phone_number(&normalized_national_number)
        {
            temp_number.italian_leading_zero = Some(true);
            if zeroes_count > 1 {
                temp_number.number_of_leading_zeros = Some(zeroes_count as i32);
            }
        }
        let number_as_int = normalized_national_number.parse::<u64>();
        match number_as_int {
            Ok(number_as_int) => temp_number.national_number = number_as_int,
            Err(err) => {
                return Err(NotANumberError::FailedToParseNumberAsInt(err).into());
            }
        }
        Ok(temp_number)
    }

    /// Checks to see if the string of characters could possibly be a phone number at
    /// all. At the moment, checks to see that the string begins with at least 3
    /// digits, ignoring any punctuation commonly found in phone numbers.  This
    /// method does not require the number to be normalized in advance - but does
    /// assume that leading non-number symbols have been removed, such as by the
    /// method `ExtractPossibleNumber`.
    pub(crate) fn is_viable_phone_number(&self, phone_number: &str) -> bool {
        if phone_number.len() < MIN_LENGTH_FOR_NSN {
            false
        } else {
            self.reg_exps
                .valid_phone_number_pattern_fullmatch
                .is_match(phone_number)
        }
    }

    /// Checks to see that the region code used is valid, or if it is not valid, that
    /// the number to parse starts with a + symbol so that we can attempt to infer
    /// the country from the number. Returns false if it cannot use the region
    /// provided and the region cannot be inferred.
    pub(crate) fn check_region_for_parsing(
        &self,
        number_to_parse: &str,
        default_region: &str,
    ) -> bool {
        self.get_metadata_for_region(default_region).is_some()
            || number_to_parse.is_empty()
            || self
                .reg_exps
                .plus_chars_pattern_start
                .is_match(number_to_parse)
    }

    /// Strips any extension (as in, the part of the number dialled after the call is
    /// connected, usually indicated with extn, ext, x or similar) from the end of
    /// the number, and returns stripped number and extension. The number passed in should be non-normalized.
    pub(crate) fn maybe_strip_extension<'a>(
        &self,
        phone_number: &'a str,
    ) -> (&'a str, Option<&'a str>) {
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
        if !self.is_viable_phone_number(phone_number_no_extn) {
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
    ///     is dialing from, if this is present in the number, and looking at the next
    ///     digits
    ///   - by stripping the '+' sign if present and then looking at the next digits
    ///   - by comparing the start of the number and the country calling code of the
    ///     default region. If the number is not considered possible for the numbering
    ///     plan of the default region initially, but starts with the country calling
    ///     code of this region, validation will be reattempted after stripping this
    ///     country calling code. If this number is considered a possible number, then
    ///     the first digits will be considered the country calling code and removed as
    ///     such.
    ///
    ///   Returns `Ok` if a country calling code was successfully
    ///   extracted or none was present, or the appropriate error otherwise, such as
    ///   if a + was present but it was not followed by a valid country calling code.
    ///   If NO_PARSING_ERROR is returned, the national_number without the country
    ///   calling code is populated, and the country_code of the phone_number passed
    ///   in is set to the country calling code if found, otherwise to 0.
    pub(crate) fn maybe_extract_country_code<'a>(
        &self,
        default_region_metadata: Option<&PhoneMetadataWrapper>,
        keep_raw_input: bool,
        national_number: &'a str,
        phone_number: &mut PhoneNumber,
    ) -> ParseResult<Cow<'a, str>> {
        // Set the default prefix to be something that will never match if there is no
        // default region.
        let possible_country_idd_prefix = default_region_metadata
            .map(|default_region_metadata| default_region_metadata.international_prefix());

        let phone_number_with_country_code_source = self
            .maybe_strip_international_prefix_and_normalize(
                national_number,
                possible_country_idd_prefix,
            )
            .map_err(|err| err.translate_internal())?;

        let national_number = phone_number_with_country_code_source.phone_number;
        if keep_raw_input {
            phone_number
                .set_country_code_source(phone_number_with_country_code_source.country_code_source);
        }
        if !matches!(
            phone_number_with_country_code_source.country_code_source,
            CountryCodeSource::FromDefaultCountry
        ) {
            if national_number.len() <= MIN_LENGTH_FOR_NSN {
                trace!(
                    "Phone number {} had an IDD, but after this was not \
                long enough to be a viable phone number.",
                    national_number
                );
                return Err(ParseError::TooShortAfterIdd.into());
            }
            let Some((national_number, potential_country_code)) =
                self.extract_country_code(national_number)
            else {
                // If this fails, they must be using a strange country calling code that we
                // don't recognize, or that doesn't exist.
                return Err(ParseError::InvalidCountryCode.into());
            };
            phone_number.country_code = potential_country_code;
            return Ok(national_number);
        } else if let Some(default_region_metadata) = default_region_metadata {
            // Check to see if the number starts with the country calling code for the
            // default region. If so, we remove the country calling code, and do some
            // checks on the validity of the number before and after.
            let default_country_code = default_region_metadata.original.country_code();
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
                    )
                    .map_err(|err| err.translate_internal())?;

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
                )? && helper_functions::is_match(
                    &self.matcher_api,
                    &potential_national_number,
                    general_num_desc,
                )?) || test_number_length_with_unknown_type(
                    &national_number,
                    default_region_metadata,
                )
                .is_err_and(|e| matches!(e, ValidationError::TooLong))
                {
                    if keep_raw_input {
                        phone_number
                            .set_country_code_source(CountryCodeSource::FromNumberWithoutPlusSign);
                    }
                    phone_number.country_code = default_country_code;
                    return Ok(potential_national_number);
                }
            }
        }
        // No country calling code present. Set the country_code to 0.
        phone_number.country_code = 0;
        Ok(national_number)
    }

    /// Gets an example of a valid phone number for a given region.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region for which to get an example number.
    pub(crate) fn get_example_number(&self, region_code: &str) -> ExampleNumberResult {
        self.get_example_number_for_type_and_region_code(region_code, PhoneNumberType::FixedLine)
    }

    /// Gets an example of an invalid phone number for a given region.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region for which to get an invalid example number.
    pub(crate) fn get_invalid_example_number(&self, region_code: &str) -> ExampleNumberResult {
        let Some(region_metadata) = self.region_to_metadata_map.get(region_code) else {
            warn!("Invalid or unknown region code ({}) provided.", region_code);
            return Err(GetExampleNumberError::InvalidRegionCode.into());
        };

        // We start off with a valid fixed-line number since every country supports
        // this. Alternatively we could start with a different number type, since
        // fixed-line numbers typically have a wide breadth of valid number lengths
        // and we may have to make it very short before we get an invalid number.
        let desc = get_number_desc_by_type(region_metadata, PhoneNumberType::FixedLine);

        if desc.original.example_number.is_none() {
            // This shouldn't happen - we have a test for this.
            return Err(GetExampleNumberError::NoExampleNumber.into());
        }

        let example_number = desc.original.example_number();
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
            let Ok(possibly_valid_number) = self.parse(number_to_try, Some(region_code)) else {
                continue;
            };
            // We don't check the return value since we have already checked the
            // length, we know example numbers have only valid digits, and we know the
            // region code is fine.
            if !self
                .is_valid_number(&possibly_valid_number)
                .map_err(|err| err.translate_internal())?
            {
                return Ok(possibly_valid_number);
            }
        }
        // We have a test to check that this doesn't happen for any of our supported
        Err(GetExampleNumberError::CouldNotGetNumber.into())
    }

    /// Gets an example of a valid phone number for a given region and type.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region for which to get an example number.
    /// * `number_type` - The type of number to get an example for.
    pub(crate) fn get_example_number_for_type_and_region_code(
        &self,
        region_code: &str,
        phone_number_type: PhoneNumberType,
    ) -> ExampleNumberResult {
        let Some(region_metadata) = self.region_to_metadata_map.get(region_code) else {
            warn!("Invalid or unknown region code ({}) provided.", region_code);
            return Err(GetExampleNumberError::InvalidRegionCode.into());
        };
        let desc = get_number_desc_by_type(region_metadata, phone_number_type);
        if desc.original.example_number.is_some() {
            return self
                .parse(desc.original.example_number(), Some(region_code))
                .inspect_err(|err| error!("Error parsing example number ({:?})", err))
                .map_err(|err| err.translate());
        }
        Err(GetExampleNumberError::CouldNotGetNumber.into())
    }

    /// Gets an example of a valid phone number for a given region and type.
    ///
    /// # Arguments
    ///
    /// * `region_code` - The region for which to get an example number.
    /// * `phone_number_type` - The type of number to get an example for.
    pub(crate) fn get_example_number_for_type(
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
                    return Some(Err(GetExampleNumberError::InvalidRegionCode.into()));
                };
                let desc = get_number_desc_by_type(metadata, phone_number_type);
                if desc.original.example_number.is_some() {
                    let mut buf = itoa::Buffer::new();
                    return Some(
                        self.parse(
                            &fast_cat::concat_str!(
                                PLUS_SIGN,
                                buf.format(country_calling_code),
                                desc.original.example_number()
                            ),
                            None,
                        )
                        .map_err(|err| err.translate::<GetExampleNumberError>()),
                    );
                }
                None
            })
        {
            return res;
        }
        // There are no example numbers of this type for any country in the library.
        Err(GetExampleNumberError::CouldNotGetNumber.into())
    }

    /// Gets an example of a valid phone number for a non-geographical entity.
    ///
    /// # Arguments
    ///
    /// * `country_calling_code` - The non-geographical country calling code.
    pub(crate) fn get_example_number_for_non_geo_entity(
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
            return Err(GetExampleNumberError::InvalidRegionCode.into());
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
            if number_type.original.example_number.is_none() {
                continue;
            }
            let mut buf = itoa::Buffer::new();
            return self
                .parse(
                    &fast_cat::concat_str!(
                        PLUS_SIGN,
                        buf.format(country_calling_code),
                        number_type.original.example_number(),
                    ),
                    None,
                )
                .map_err(|err| err.translate());
        }
        Err(GetExampleNumberError::CouldNotGetNumber.into())
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
    pub(crate) fn maybe_strip_international_prefix_and_normalize<'a>(
        &self,
        phone_number: &'a str,
        possible_idd_prefix: Option<&RegexTriplets>,
    ) -> RegexResult<PhoneNumberWithCountryCodeSource<'a>> {
        if phone_number.is_empty() {
            Ok(PhoneNumberWithCountryCodeSource::new(
                Cow::Borrowed(phone_number),
                CountryCodeSource::FromDefaultCountry,
            ))
        } else if let Some(plus_match) = self.reg_exps.plus_chars_pattern_start.find(phone_number) {
            let number_string_piece = &phone_number[plus_match.end()..];
            // Can now normalize the rest of the number since we've consumed the "+"
            // sign at the start.
            Ok(PhoneNumberWithCountryCodeSource::new(
                Cow::Owned(self.normalize(number_string_piece)),
                CountryCodeSource::FromNumberWithPlusSign,
            ))
        } else {
            // Attempt to parse the first digits as an international prefix.
            let normalized_number = self.normalize(phone_number);
            let value = if let Some(idd_prefix) = possible_idd_prefix
                && let Some(stripped_prefix_number) =
                    self.parse_prefix_as_idd(&normalized_number, idd_prefix.anchor_start()?)
            {
                PhoneNumberWithCountryCodeSource::new(
                    Cow::Owned(stripped_prefix_number.to_owned()),
                    CountryCodeSource::FromNumberWithIdd,
                )
            } else {
                PhoneNumberWithCountryCodeSource::new(
                    Cow::Owned(normalized_number),
                    CountryCodeSource::FromDefaultCountry,
                )
            };

            Ok(value)
        }
    }

    /// Normalizes a string of characters representing a phone number. This performs
    /// the following conversions:
    ///   - Punctuation is stripped.
    ///   
    /// For ALPHA/VANITY numbers:
    ///   - Letters are converted to their numeric representation on a telephone
    ///     keypad. The keypad used here is the one defined in ITU Recommendation
    ///     E.161. This is only done if there are 3 or more letters in the number, to
    ///     lessen the risk that such letters are typos.
    ///
    /// For other numbers:
    ///   - Wide-ascii digits are converted to normal ASCII (European) digits.
    ///   - Arabic-Indic numerals are converted to European numerals.
    ///   - Spurious alpha characters are stripped.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - Number to normalize.
    pub(crate) fn normalize(&self, phone_number: &str) -> String {
        if self
            .reg_exps
            .valid_alpha_phone_pattern_fullmatch
            .is_match(phone_number)
        {
            normalize_helper(&self.reg_exps.alpha_phone_mappings, true, phone_number)
        } else {
            self.normalize_digits_only(phone_number)
        }
    }

    /// Strips the IDD from the start of the number if present. Helper function used
    /// by MaybeStripInternationalPrefixAndNormalize.
    pub(crate) fn parse_prefix_as_idd<'a>(
        &self,
        phone_number: &'a str,
        idd_pattern_start: Option<&Regex>,
    ) -> Option<&'a str> {
        let idd_pattern_start = idd_pattern_start?;
        // First attempt to strip the idd_pattern at the start, if present. We make a
        // copy so that we can revert to the original string if necessary.
        let idd_pattern_match = idd_pattern_start.find(phone_number)?;
        let captured_range_end = idd_pattern_match.end();

        // Only strip this if the first digit after the match is not a 0, since
        // country calling codes cannot begin with 0.
        if phone_number[captured_range_end..]
            .chars()
            .find_map(uniprops_digits::uniprops::get_digit_value)
            .is_some_and(|d| d == 0)
        {
            return None;
        }
        Some(&phone_number[captured_range_end..])
    }

    /// Checks if a phone number is geographical.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    pub(crate) fn is_number_geographical(&self, phone_number: &PhoneNumber) -> RegexResult<bool> {
        Ok(self.is_number_geographical_by_country_code_and_type(
            self.get_number_type(phone_number)?,
            phone_number.country_code,
        ))
    }

    pub(crate) fn is_number_geographical_by_country_code_and_type(
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

    /// Gets the length of the geographical area code for a given number.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    pub(crate) fn get_length_of_geographical_area_code(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<usize> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) =
            region_code.and_then(|region_code| self.region_to_metadata_map.get(region_code))
        else {
            return Ok(0);
        };

        let phone_number_type = self.get_number_type(phone_number)?;
        let country_calling_code = phone_number.country_code;

        // If a country doesn't use a national prefix, and this number doesn't have an
        // Italian leading zero, we assume it is a closed dialling plan with no area
        // codes.
        // Note:this is our general assumption, but there are exceptions which are
        // tracked in COUNTRIES_WITHOUT_NATIONAL_PREFIX_WITH_AREA_CODES.
        if metadata.original.national_prefix.is_none()
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
                .contains(&country_calling_code))
        {
            return Ok(0);
        }

        if !self.is_number_geographical_by_country_code_and_type(
            phone_number_type,
            country_calling_code,
        ) {
            return Ok(0);
        }

        self.get_length_of_national_destination_code(phone_number)
    }

    /// Gets the length of the national destination code for a given number.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number to check.
    pub(crate) fn get_length_of_national_destination_code(
        &self,
        phone_number: &PhoneNumber,
    ) -> RegexResult<usize> {
        let mut copied_proto = phone_number.clone();
        if phone_number.extension.is_some() {
            // Clear the extension so it's not included when formatting.
            copied_proto.extension = None;
        }

        let formatted_number = self.format(&copied_proto, PhoneNumberFormat::International)?;

        const ITERATIONS_COUNT: usize = 3;
        let mut captured_groups = [0; ITERATIONS_COUNT];
        let (ndc_index, third_group) = (1, 2);
        let mut capture_iter = self
            .reg_exps
            .capturing_ascii_digits_pattern
            .captures_iter(&formatted_number);
        for captured_group in captured_groups.iter_mut() {
            if let Some(matches) = capture_iter.next().and_then(|captures| captures.get(1)) {
                *captured_group = matches.len();
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
            if let Some(mobile_token) = self.get_country_mobile_token(phone_number.country_code) {
                return Ok(captured_groups[third_group] + mobile_token.len_utf8());
            }
        }
        Ok(captured_groups[ndc_index])
    }

    pub(crate) fn get_country_mobile_token(&self, country_calling_code: i32) -> Option<char> {
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
    pub(crate) fn extract_country_code<'a>(
        &self,
        national_number: Cow<'a, str>,
    ) -> Option<(Cow<'a, str>, i32)> {
        if national_number.as_ref().is_empty() || national_number.as_ref().starts_with('0') {
            return None;
        }
        for i in 0..=MAX_LENGTH_COUNTRY_CODE {
            let Ok(potential_country_code) = national_number.as_ref()[0..i].parse::<i32>() else {
                continue;
            };
            let region_code = self.get_region_code_for_country_code(potential_country_code);
            if region_code.is_some() {
                return match national_number {
                    Cow::Borrowed(s) => Some((Cow::Borrowed(&s[i..]), potential_country_code)),
                    Cow::Owned(mut s) => {
                        s.drain(0..i);
                        Some((Cow::Owned(s), potential_country_code))
                    }
                };
            }
        }
        None
    }

    // Strips any national prefix (such as 0, 1) present in the number provided.
    // The number passed in should be the normalized telephone number that we wish
    // to strip any national dialing prefix from. The metadata should be for the
    // region that we think this number is from. Returns true if a national prefix
    // and/or carrier code was stripped.
    pub(crate) fn maybe_strip_national_prefix_and_carrier_code<'a>(
        &self,
        metadata: &PhoneMetadataWrapper,
        phone_number: &'a str,
    ) -> RegexResult<(Cow<'a, str>, Option<&'a str>)> {
        let Some(possible_national_prefix_pattern) = metadata
            .national_prefix_for_parsing()
            .anchor_start()?
            .take_if(|_| !phone_number.is_empty())
        else {
            // Early return for numbers of zero length or with no national prefix
            // possible.
            return Ok((phone_number.into(), None));
        };
        let general_desc = &metadata.general_desc;
        // Check if the original number is viable.
        let is_viable_original_number =
            helper_functions::is_match(&self.matcher_api, phone_number, general_desc)?;
        // Attempt to parse the first digits as a national prefix. We make a
        // copy so that we can revert to the original string if necessary.
        let transform_rule = metadata.original.national_prefix_transform_rule();

        let captures = possible_national_prefix_pattern.captures(phone_number);
        let first_capture = captures.as_ref().and_then(|c| c.get(1));
        let second_capture = captures.as_ref().and_then(|c| c.get(2));

        let condition = |first_capture: &crate::regexp::Match<'_>| {
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
                possible_national_prefix_pattern.replace(phone_number, transform_rule);
            if is_viable_original_number
                && !helper_functions::is_match(&self.matcher_api, &replaced_number, general_desc)?
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
                && !helper_functions::is_match(&self.matcher_api, stripped_number, general_desc)?
            {
                return Ok((phone_number.into(), None));
            }
            let carrier_code_temp = first_capture.map(|capture| capture.as_str());

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
    pub(crate) fn get_italian_leading_zeros_for_phone_number(
        national_number: &str,
    ) -> Option<usize> {
        if national_number.len() < 2 {
            return None;
        }
        let zero_count = national_number.chars().take_while(|c| *c == '0').count();
        if zero_count == 0 {
            return None;
        }
        // Note that if the national number is all "0"s, the last "0" is not
        // counted as a leading zero.
        if zero_count == national_number.len() {
            return Some(zero_count - 1);
        }

        Some(zero_count)
    }

    /// Converts all alpha characters in a phone number string to their respective digits on a keypad.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The phone number string with alpha characters.
    pub(crate) fn convert_alpha_characters_in_number(&self, phone_number: &str) -> String {
        normalize_helper(&self.reg_exps.alpha_phone_mappings, false, phone_number)
    }

    /// Checks whether two phone numbers match.
    /// Returns the type of match.
    ///
    /// # Arguments
    ///
    /// * `number1` - The first phone number to compare.
    /// * `number2` - The second phone number to compare.
    pub(crate) fn is_number_match(
        &self,
        first_number_in: &PhoneNumber,
        second_number_in: &PhoneNumber,
    ) -> MatchType {
        // Early exit if both had extensions and these are different.
        if first_number_in.extension.is_some()
            && second_number_in.extension.is_some()
            && first_number_in.extension() != second_number_in.extension()
        {
            return MatchType::NoMatch;
        }

        // We only are about the fields that uniquely define a number, so we copy
        // these across explicitly.
        let mut first_number = copy_core_fields_only(first_number_in);
        let second_number = copy_core_fields_only(second_number_in);

        let first_number_country_code = first_number.country_code;
        let second_number_country_code = second_number.country_code;
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
        first_number.country_code = second_number_country_code;
        // If all else was the same, then this is an NSN_MATCH.
        if first_number == second_number {
            return MatchType::NsnMatch;
        }
        if is_national_number_suffix_of_the_other(&first_number, &second_number) {
            return MatchType::ShortNsnMatch;
        }
        MatchType::NoMatch
    }

    /// Checks whether two phone numbers match.
    /// Returns the type of match.
    ///
    /// # Arguments
    ///
    /// * `number1` - The first phone number to compare.
    /// * `number2` - The second phone number to compare.
    pub(crate) fn is_number_match_with_two_strings(
        &self,
        first_number: &str,
        second_number: &str,
    ) -> MatchResult {
        match self.parse(first_number, None) {
            Ok(first_number_as_proto) => {
                return self.is_number_match_with_one_string(&first_number_as_proto, second_number);
            }
            Err(err) => {
                if !matches!(err, InternalError::Wrapped(ParseError::InvalidCountryCode)) {
                    return Err(err.translate());
                }
            }
        }
        match self.parse(second_number, None) {
            Ok(second_number_as_proto) => {
                self.is_number_match_with_one_string(&second_number_as_proto, first_number)
            }
            Err(err) => {
                if !matches!(err, InternalError::Wrapped(ParseError::InvalidCountryCode)) {
                    return Err(err.translate());
                }
                let first_number_as_proto = self
                    .parse_helper(first_number, None, false, false)
                    .map_err(|err| err.translate())?;
                let second_number_as_proto = self
                    .parse_helper(second_number, None, false, false)
                    .map_err(|err| err.translate())?;
                Ok(self.is_number_match(&first_number_as_proto, &second_number_as_proto))
            }
        }
    }

    /// Checks whether two phone numbers match.
    /// Returns the type of match.
    ///
    /// # Arguments
    ///
    /// * `number1` - The first phone number to compare.
    /// * `number2` - The second phone number to compare.
    pub(crate) fn is_number_match_with_one_string(
        &self,
        first_number: &PhoneNumber,
        second_number: &str,
    ) -> MatchResult {
        // First see if the second number has an implicit country calling code, by
        // attempting to parse it.
        match self.parse(second_number, None) {
            Ok(second_number_as_proto) => {
                return Ok(self.is_number_match(first_number, &second_number_as_proto));
            }
            Err(err) => {
                if !matches!(err, InternalError::Wrapped(ParseError::InvalidCountryCode)) {
                    return Err(err.translate());
                }
            }
        }
        // The second number has no country calling code. EXACT_MATCH is no longer
        // possible.  We parse it as if the region was the same as that for the
        // first number, and if EXACT_MATCH is returned, we replace this with
        // NSN_MATCH.
        let first_number_region = self.get_region_code_for_country_code(first_number.country_code);
        if let Some(first_number_region) = first_number_region {
            let second_number_with_first_number_region = self
                .parse(second_number, Some(first_number_region))
                .map_err(|err| err.translate())?;
            Ok(
                match self.is_number_match(first_number, &second_number_with_first_number_region) {
                    MatchType::ExactMatch => MatchType::NsnMatch,
                    m => m,
                },
            )
        } else {
            // If the first number didn't have a valid country calling code, then we
            // parse the second number without one as well.
            let second_number_as_proto = self
                .parse_helper(second_number, None, false, false)
                .map_err(|err| err.translate())?;
            Ok(self.is_number_match(first_number, &second_number_as_proto))
        }
    }

    /// Checks if a string contains alpha characters, which suggests it is a vanity number.
    ///
    /// # Arguments
    ///
    /// * `phone_number` - The string to check.
    pub(crate) fn is_alpha_number(&self, phone_number: &str) -> bool {
        if !self.is_viable_phone_number(phone_number) {
            // Number is too short, or doesn't match the basic phone number pattern.
            return false;
        }
        // Copy the number, since we are going to try and strip the extension from it.
        let (number, _extension) = self.maybe_strip_extension(phone_number);
        self.reg_exps
            .valid_alpha_phone_pattern_fullmatch
            .is_match(number)
    }
}
