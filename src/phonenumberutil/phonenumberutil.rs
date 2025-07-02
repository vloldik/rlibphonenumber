use std::{
    borrow::Cow,
    cell::LazyCell,
    cmp::max,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
    sync::Arc,
};

use super::phone_number_regexps_and_mappings::PhoneNumberRegExpsAndMappings;
use crate::{
    i18n, interfaces::MatcherApi, macros::owned_from_cow_or, phonenumberutil::{
        errors::PhoneNumberUtilError, helper_constants::{
            DEFAULT_EXTN_PREFIX, NANPA_COUNTRY_CODE, PLUS_SIGN, REGION_CODE_FOR_NON_GEO_ENTITY,
            RFC3966_EXTN_PREFIX, RFC3966_PREFIX, VALID_PUNCTUATION,
        }, helper_functions::{
            self, get_supported_types_for_metadata, load_compiled_metadata, normalize_helper, populate_supported_types_for_metadata, prefix_number_with_country_calling_code, test_number_length, test_number_length_with_unknown_type
        }, PhoneNumberFormat, PhoneNumberType, ValidationResultErr
    }, proto_gen::{
        phonemetadata::{NumberFormat, PhoneMetadata, PhoneMetadataCollection, PhoneNumberDesc},
        phonenumber::PhoneNumber,
    }, regex_based_matcher::RegexBasedMatcher, regex_util::{RegexConsume, RegexFullMatch}
};

use log::{trace, warn};

// Helper type for Result
pub type Result<T> = std::result::Result<T, PhoneNumberUtilError>;

pub struct PhoneNumberUtil {
    /// An API for validation checking.
    matcher_api: Box<dyn MatcherApi>,

    /// Helper class holding useful regular expressions and character mappings.
    reg_exps: PhoneNumberRegExpsAndMappings,

    /// A mapping from a country calling code to a RegionCode object which denotes
    /// the region represented by that country calling code. Note regions under
    /// NANPA share the country calling code 1 and Russia and Kazakhstan share the
    /// country calling code 7. Under this map, 1 is mapped to region code "US" and
    /// 7 is mapped to region code "RU". This is implemented as a sorted vector to
    /// achieve better performance.
    country_calling_code_to_region_code_map: Vec<(i32, Vec<String>)>,

    /// The set of regions that share country calling code 1.
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
    pub(super) fn new() -> Self {
        let mut instance = Self {
            matcher_api: Box::new(RegexBasedMatcher::new()),
            reg_exps: PhoneNumberRegExpsAndMappings::new(),
            country_calling_code_to_region_code_map: Default::default(),
            nanpa_regions: Default::default(),
            region_to_metadata_map: Default::default(),
            country_code_to_non_geographical_metadata_map: Default::default(),
        };
        let metadata_collection = match load_compiled_metadata() {
            Err(err) => {
                let err_message = format!("Could not parse compiled-in metadata: {:?}", err);
                log::error!("{}", err_message);
                panic!("{}", err_message);
            }
            Ok(metadata) => metadata,
        };
        // Storing data in a temporary map to make it easier to find other regions
        // that share a country calling code when inserting data.
        let mut country_calling_code_to_region_map = HashMap::<i32, VecDeque<String>>::new();
        for metadata in metadata_collection.metadata {
            let region_code = &metadata.id().to_string();
            let main_country_code = metadata.main_country_for_code();
            if i18n::RegionCode::get_unknown() == region_code {
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
            country_calling_code_to_region_map.into_iter().map(| (k, v) | {
                (k, Vec::from(v))
            })
        );
        // Sort all the pairs in ascending order according to country calling code.
        instance
            .country_calling_code_to_region_code_map
            .sort_by_key(|(a, _)| *a);
        instance
    }

    pub fn get_supported_regions(&self) -> Vec<&str> {
        let mut regions = Vec::new();
        for (k, _) in self.region_to_metadata_map.iter() {
            regions.push(k.as_str());
        }
        regions
    }

    pub fn get_supported_global_network_calling_codes(&self) -> HashSet<i32> {
        let mut codes = HashSet::new();
        for (k, _) in self.country_code_to_non_geographical_metadata_map.iter() {
            codes.insert(*k);
        }
        codes
    }

    pub fn get_supported_calling_codes(&self) -> HashSet<i32> {
        let mut codes = HashSet::new();

        for (k, _) in self.country_calling_code_to_region_code_map.iter() {
            codes.insert(*k);
        }
        codes
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

    fn get_extn_patterns_for_matching(&self) -> &str {
        return &self.reg_exps.extn_patterns_for_matching;
    }

    fn starts_with_plus_chars_pattern(&self, phone_number: &str) -> bool {
        self.reg_exps
            .plus_chars_pattern
            .consume_start(phone_number)
            .is_some()
    }

    fn contains_only_valid_digits(&self, s: &str) -> bool {
        self.reg_exps.digits_pattern.full_match(s)
    }

    fn trim_unwanted_end_chars(&self, phone_number: &mut String) {
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
            phone_number.truncate(new_len);
        }
    }

    fn is_format_eligible_for_as_you_type_formatter(&self, format: &str) -> bool {
        // We require that the first
        // group is present in the output pattern to ensure no data is lost while
        // formatting; when we format as you type, this should always be the case.
        return self
            .reg_exps
            .is_format_eligible_as_you_type_formatting_regex
            .full_match(format);
    }

    fn formatting_rule_has_first_group_only(&self, national_prefix_formatting_rule: &str) -> bool {
        // A pattern that is used to determine if the national prefix formatting rule
        // has the first group only, i.e., does not start with the national prefix.
        // Note that the pattern explicitly allows for unbalanced parentheses.
        let first_group_only_prefix_pattern = self
            .reg_exps
            .regexp_cache
            .get_regex("\\(?\\$1\\)?")
            .expect("Invalid constant pattern!");
        return national_prefix_formatting_rule.is_empty()
            || first_group_only_prefix_pattern.full_match(national_prefix_formatting_rule);
    }

    fn get_ndd_prefix_for_region(
        &self,
        region_code: &str,
        strip_non_digits: bool,
    ) -> Option<String> {
        self.region_to_metadata_map
            .get(region_code)
            .and_then(|metadata| {
                let mut prefix = metadata.national_prefix().to_owned();
                if strip_non_digits {
                    prefix = prefix.replace("~", "");
                }
                Some(prefix)
            })
            .or_else(|| {
                warn!("Invalid or unknown region code ({}) provided.", region_code);
                None
            })
    }

    fn is_valid_region_code(&self, region_code: &str) -> bool {
        return self.region_to_metadata_map.contains_key(region_code);
    }

    // TODO: Uncomment
    fn format<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        number_format: PhoneNumberFormat,
    ) -> Result<Cow<'b, str>> {
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
        let mut formatted_number = Self::get_national_significant_number(phone_number);

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
        // that to format NANPA numbers. The same applies to Russian Fed regions -
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

    fn get_national_significant_number(phone_number: &PhoneNumber) -> String {
        let zeros_start = if phone_number.italian_leading_zero() {
            "0".repeat(max(phone_number.number_of_leading_zeros() as usize, 0))
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
    fn get_region_code_for_country_code(&self, country_calling_code: i32) -> &str {
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        return region_codes
            .first()
            .map(|v| v.clone())
            .unwrap_or(i18n::RegionCode::get_unknown());
    }

    // Returns the region codes that matches the specific country calling code. In
    // the case of no region code being found, region_codes will be left empty.
    fn get_region_codes_for_country_calling_code(
        &self,
        country_calling_code: i32,
    ) -> Vec<&str> {
        let mut acc = Vec::with_capacity(10);
        // Create a IntRegionsPair with the country_code passed in, and use it to
        // locate the pair with the same country_code in the sorted vector.
        self.country_calling_code_to_region_code_map
            .binary_search_by_key(&country_calling_code, |(code, _)| *code)
            .map(|index| {
                self.country_calling_code_to_region_code_map[index]
                    .1
                    .iter()
                    .for_each(|v| {
                        acc.push(v.as_str());
                    });
            }) /* suppress Result ignoring */
            .ok();
        acc
    }

    fn get_metadata_for_region_or_calling_code(
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

    /// TODO: uncomment
    fn format_nsn<'b>(
        &self,
        phone_number: &'b str,
        metadata: &PhoneMetadata,
        number_format: PhoneNumberFormat,
    ) -> Result<Cow<'b, str>> {
        self.format_nsn_with_carrier(phone_number, metadata, number_format, "")
    }

    fn format_nsn_with_carrier<'b>(
        &self,
        number: &'b str,
        metadata: &PhoneMetadata,
        number_format: PhoneNumberFormat,
        carrier_code: &str,
    ) -> Result<Cow<'b, str>> {
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

    fn choose_formatting_pattern_for_number<'b>(
        &self,
        available_formats: &'b [NumberFormat],
        national_number: &str,
    ) -> Result<Option<&'b NumberFormat>> {
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
                        .and_then(|regex| Ok(regex.consume_start(national_number).is_some()))
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
    ) -> Result<Cow<'b, str>> {
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
            if let Some(rest) = self
                .reg_exps
                .separator_pattern
                .consume_start(&formatted_number)
            {
                formatted_number = Cow::Owned(rest.to_string());
            }
            // Then replace all separators with a "-".
            // Rust note: if cow::Borrowed returned number not changed
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
    fn format_nsn_using_pattern<'b>(
        &self,
        national_number: &'b str,
        formatting_pattern: &NumberFormat,
        number_format: PhoneNumberFormat,
    ) -> Result<Cow<'b, str>> {
        self.format_nsn_using_pattern_with_carrier(
            national_number,
            formatting_pattern,
            number_format,
            "",
        )
    }

    // Returns the formatted extension of a phone number, if the phone number had an
    // extension specified else None.
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

    fn format_by_pattern(
        &self,
        phone_number: &PhoneNumber,
        number_format: PhoneNumberFormat,
        user_defined_formats: &[NumberFormat],
    ) -> Result<String> {
        let country_calling_code = phone_number.country_code();
        // Note GetRegionCodeForCountryCode() is used because formatting information
        // for regions which share a country calling code is contained by only one
        // region for performance reasons. For example, for NANPA regions it will be
        // contained in the metadata for US.
        let national_significant_number = Self::get_national_significant_number(phone_number);
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

    fn format_national_number_with_carrier_code(
        &self,
        phone_number: &PhoneNumber,
        carrier_code: &str,
    ) -> Result<String> {
        let country_calling_code = phone_number.country_code();
        let national_significant_number = Self::get_national_significant_number(phone_number);
        let region_code = self.get_region_code_for_country_code(country_calling_code);

        // Note GetRegionCodeForCountryCode() is used because formatting information
        // for regions which share a country calling code is contained by only one
        // region for performance reasons. For example, for NANPA regions it will be
        // contained in the metadata for US.
        let Some(metadata) = self.get_metadata_for_region_or_calling_code(country_calling_code, &region_code) else {
            return Ok(national_significant_number)
        };

        let mut formatted_number = owned_from_cow_or!(
            self.format_nsn_with_carrier(
                &national_significant_number, metadata,
                PhoneNumberFormat::National, carrier_code,
            )?,
            national_significant_number
        );
        if let Some(formatted_extension) = Self::get_formatted_extension(
            phone_number, metadata, PhoneNumberFormat::National
        ) {
            formatted_number.push_str(&formatted_extension);
        }

        prefix_number_with_country_calling_code(
            country_calling_code,
            PhoneNumberFormat::National,
            &mut formatted_number,
        );

        Ok(formatted_number)
    }

    fn format_national_number_with_preferred_carrier_code(
        &self,
        phone_number: &PhoneNumber,
        fallback_carrier_code: &str,
    ) -> Result<String> {
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

    fn format_number_for_mobile_dialing<'b>(
        &self,
        phone_number: &'b PhoneNumber,
        calling_from: &str,
        with_formatting: bool,
    ) -> Result<Cow<'b, str>> {
        let country_calling_code = phone_number.country_code();
        if !self.has_valid_country_calling_code(country_calling_code) {
            return if phone_number.has_raw_input() {
                Ok(Cow::Borrowed(phone_number.raw_input()))
            } else {
                Ok(Cow::Borrowed(""))
            }
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
            let is_fixed_line_or_mobile = matches!(number_type, 
                PhoneNumberType::FixedLine | PhoneNumberType::FixedLineOrMobile | PhoneNumberType::Mobile
            );
            // Carrier codes may be needed in some countries. We handle this here.
            if (region_code == "BR") && (is_fixed_line_or_mobile) {
                // Historically, we set this to an empty string when parsing with raw
                // input if none was found in the input string. However, this doesn't
                // result in a number we can dial. For this reason, we treat the empty
                // string the same as if it isn't set at all.
                if !number_no_extension.preferred_domestic_carrier_code().is_empty() {
                    formatted_number = self.format_national_number_with_preferred_carrier_code(
                        &number_no_extension, ""
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
                let region_metadata = self.region_to_metadata_map
                    .get(calling_from)
                    .unwrap() /* we've checked if number is valid at top of function */;
                let national_number = Self::get_national_significant_number(&number_no_extension);
                let format = if self.can_be_internationally_dialled(&number_no_extension)? 
                    && test_number_length_with_unknown_type(
                        &national_number, 
                        region_metadata
                ).is_err_and(| e | matches!(e, ValidationResultErr::TooShort)) {
                    PhoneNumberFormat::International
                }
                else {
                    PhoneNumberFormat::National
                };
                if let Cow::Owned(s) = self.format(
                    &number_no_extension, format
                )? {
                    formatted_number = s;
                }
            }
            else {
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
                        is_fixed_line_or_mobile)) &&
                    self.can_be_internationally_dialled(&number_no_extension)? {
                    PhoneNumberFormat::International
                }
                else {
                    PhoneNumberFormat::National
                };
                if let Cow::Owned(s) = self.format(
                    &number_no_extension, format
                )? {
                    formatted_number = s;
                }
            }
        }
        else if is_valid_number && self.can_be_internationally_dialled(&number_no_extension)? {
            // We assume that short numbers are not diallable from outside their
            // region, so if a number is not a valid regular length phone number, we
            // treat it as if it cannot be internationally dialled.
            let format = if with_formatting {
                PhoneNumberFormat::International
            } else {
                PhoneNumberFormat::E164
            };
            return Ok(Cow::Owned(
                owned_from_cow_or!(self.format(
                    &number_no_extension, format
                )?, formatted_number))
            )
        }
        if !with_formatting {
            Ok(Cow::Owned(self.normalize_diallable_chars_only(&formatted_number)))
        } else {
            Ok(Cow::Owned(formatted_number))
        }
    }

    fn get_number_type(&self, phone_number: &PhoneNumber) -> Result<PhoneNumberType> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) = self
            .get_metadata_for_region_or_calling_code(phone_number.country_code(), region_code)
        else {
            return Ok(PhoneNumberType::Unknown)
        };
        let national_significant_number = Self::get_national_significant_number(phone_number);
        Ok(self.get_number_type_helper(&national_significant_number, metadata))
    }

    fn get_region_code_for_number(&self, phone_number: &PhoneNumber) -> Result<&str>{
        let country_calling_code = phone_number.country_code();
        let region_codes = self.get_region_codes_for_country_calling_code(country_calling_code);
        if region_codes.len() == 0 {
            log::trace!("Missing/invalid country calling code ({})", country_calling_code);
            return Ok(i18n::RegionCode::get_unknown())
        }
        if region_codes.len() == 1 {
            return Ok(region_codes[0])
        } else {
            self.get_region_code_for_number_from_region_list(phone_number, &region_codes)
        }
    }

    fn get_region_code_for_number_from_region_list<'b>(
        &self,
        phone_number: &PhoneNumber, 
        region_codes: &[&'b str],
    ) -> Result<&'b str> {
        let national_number = Self::get_national_significant_number(phone_number);
        for code in region_codes {
            // Metadata cannot be NULL because the region codes come from the country
            // calling code map.
            let metadata = &self.region_to_metadata_map[*code];
            if metadata.has_leading_digits() {
                if self.reg_exps.regexp_cache
                    .get_regex(metadata.leading_digits())?
                    .consume_start(&national_number).is_some() {
                    
                    return Ok(code)
                } 
            } else if self.get_number_type_helper(&national_number, metadata) != PhoneNumberType::Unknown {
                return Ok(code);
            }
        }
        return Ok(i18n::RegionCode::get_unknown())
    }

    fn get_number_type_helper(
        &self,
        national_number: &str,
        metadata: &PhoneMetadata
    ) -> PhoneNumberType {
        if !self.is_number_matching_desc(national_number, &metadata.general_desc) {
            trace!("Number '{national_number}' type unknown - doesn't match general national number pattern");
            return PhoneNumberType::Unknown
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
                trace!("Number '{national_number}': fixed-line and mobile patterns equal,\
                 number is fixed-line or mobile");
                return PhoneNumberType::FixedLineOrMobile;
            }
            else if self.is_number_matching_desc(national_number, &metadata.mobile) {
                trace!("Number '{national_number}': Fixed-line and mobile patterns differ, but number is \
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
            && self.is_number_matching_desc(national_number, &metadata.mobile) {
            trace!("Number '{national_number}' is a mobile number.");
            return PhoneNumberType::Mobile;
        }
        trace!("Number'{national_number}' type unknown - doesn\'t match any specific number type pattern.");
        return PhoneNumberType::Unknown;
    }

    fn is_number_matching_desc(
        &self,
        national_number: &str, 
        number_desc: &PhoneNumberDesc
    ) -> bool {
        // Check if any possible number lengths are present; if so, we use them to
        // avoid checking the validation pattern if they don't match. If they are
        // absent, this means they match the general description, which we have
        // already checked before checking a specific number type.
        let actual_length = national_number.len() as i32;
        if number_desc.possible_length.len() > 0 && !number_desc.possible_length.contains(&actual_length) {
            return false;
        }
        // very common name, so specify mod
        helper_functions::is_match(&self.matcher_api, national_number, number_desc)
    }

    fn can_be_internationally_dialled(
            &self,
            phone_number: &PhoneNumber
        ) -> Result<bool> {
        let region_code = self.get_region_code_for_number(phone_number)?;
        let Some(metadata) = self.region_to_metadata_map.get(region_code) else {
            // Note numbers belonging to non-geographical entities (e.g. +800 numbers)
            // are always internationally diallable, and will be caught here.
            return Ok(true)
        };
        let national_significant_number = Self::get_national_significant_number(phone_number);
        return Ok(!self.is_number_matching_desc(
            &national_significant_number, &metadata.no_international_dialling
        ));
    }

    fn normalize_diallable_chars_only(&self, phone_number: &str) -> String {
        normalize_helper(
            &self.reg_exps.diallable_char_mappings, 
            true, phone_number
        )
    }

}
