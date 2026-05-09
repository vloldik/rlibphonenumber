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

use crate::regexp::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::phonenumberutil::{
    helper_constants::{
        CAPTURE_UP_TO_SECOND_NUMBER_START, DIGITS, MIN_LENGTH_FOR_NSN, PLUS_CHARS, PLUS_SIGN,
        RFC3966_VISUAL_SEPARATOR, STAR_SIGN, VALID_ALPHA, VALID_ALPHA_INCL_UPPERCASE,
        VALID_PUNCTUATION,
    },
    helper_functions::create_extn_pattern,
    regex_wrapper_types::RegexTriplets,
};

#[allow(unused)]
pub(crate) struct PhoneNumberRegExpsAndMappings {
    /// A map that contains characters that are essential when dialling. That means
    /// any of the characters in this map must not be removed from a number when
    /// dialing, otherwise the call will not reach the intended destination.
    pub diallable_char_mappings: FxHashMap<char, char>,
    /// These mappings map a character (key) to a specific digit that should
    /// replace it for normalization purposes.
    pub alpha_mappings: FxHashMap<char, char>,
    /// For performance reasons, store a map of combining alpha_mappings with ASCII
    /// digits.
    pub alpha_phone_mappings: FxHashMap<char, char>,

    /// Separate map of all symbols that we wish to retain when formatting alpha
    /// numbers. This includes digits, ascii letters and number grouping symbols
    /// such as "-" and " ".
    pub all_plus_number_grouping_symbols: FxHashMap<char, char>,

    /// Map of country calling codes that use a mobile token before the area code.
    /// One example of when this is relevant is when determining the length of the
    /// national destination code, which should be the length of the area code plus
    /// the length of the mobile token.
    pub mobile_token_mappings: FxHashMap<i32, char>,

    /// Set of country codes that doesn't have national prefix, but it has area
    /// codes.
    pub countries_without_national_prefix_with_area_codes: FxHashSet<i32>,

    /// Set of country codes that have geographically assigned mobile numbers (see
    /// geo_mobile_countries_ below) which are not based on *area codes*. For
    /// example, in China mobile numbers start with a carrier indicator, and beyond
    /// that are geographically assigned: this carrier indicator is not considered
    /// to be an area code.
    pub geo_mobile_countries_without_mobile_area_codes: FxHashSet<i32>,

    /// Set of country calling codes that have geographically assigned mobile
    /// numbers. This may not be complete; we add calling codes case by case, as we
    /// find geographical mobile numbers or hear from user reports.
    pub geo_mobile_countries: FxHashSet<i32>,

    /// Pattern that makes it easy to distinguish whether a region has a single
    /// international dialing prefix or not. If a region has a single international
    /// prefix (e.g. 011 in USA), it will be represented as a string that contains
    /// a sequence of ASCII digits, and possibly a tilde, which signals waiting for
    /// the tone. If there are multiple available international prefixes in a
    /// region, they will be represented as a regex string that always contains one
    /// or more characters that are not ASCII digits or a tilde.
    pub single_international_prefix_fullmatch: Regex,

    pub capturing_ascii_digits_pattern: Regex,

    /// Regular expression of valid characters before a marker that might indicate
    /// a second number.
    pub capture_up_to_second_number_start_pattern: Regex,

    /// Regular expression of groups of valid punctuation characters.
    pub separator_pattern_anchor_start: Regex,
    pub separator_pattern: Regex,

    /// Regexp of all known extension prefixes used by different regions followed
    /// by 1 or more valid digits, for use when parsing.
    pub extn_pattern: Regex,

    /// We append optionally the extension pattern to the end here, as a valid
    /// phone number may have an extension prefix appended, followed by 1 or more
    /// digits.
    pub valid_phone_number_pattern_fullmatch: Regex,

    /// We use this pattern to check if the phone number has at least three letters
    /// in it - if so, then we treat it as a number where some phone-number digits
    /// are represented by letters.
    pub valid_alpha_phone_pattern_fullmatch: Regex,

    pub first_group_capturing_pattern: Regex,

    pub carrier_code_pattern: Regex,

    pub plus_chars_pattern_start: Regex,

    /// Regular expression of valid global-number-digits for the phone-context
    /// parameter, following the syntax defined in RFC3966.
    pub rfc3966_global_number_digits_pattern_fullmatch: Regex,

    /// Regular expression of valid domainname for the phone-context parameter,
    /// following the syntax defined in RFC3966.
    pub rfc3966_domainname_pattern_fullmatch: Regex,

    /// *Rust note*: It's for some reason calculated inside function in C++,
    /// so, we move it here
    ///
    /// A pattern that is used to determine if a numberFormat under
    /// availableFormats is eligible to be used by the AYTF. It is eligible when
    /// the format element under numberFormat contains groups of the dollar sign
    /// followed by a single digit, separated by valid phone number punctuation.
    /// This prevents invalid punctuation (such as the star sign in Israeli star
    /// numbers) getting into the output of the AYTF.
    pub is_format_eligible_as_you_type_formatting_regex_fullmatch: Regex,

    /// Added for function `formatting_rule_has_first_group_only`
    /// A pattern that is used to determine if the national prefix formatting rule
    /// has the first group only, i.e., does not start with the national prefix.
    /// Note that the pattern explicitly allows for unbalanced parentheses.
    pub formatting_rule_has_first_group_only_regex_fullmatch: Regex,

    pub catch_all_formatting_regex: RegexTriplets,
}

impl PhoneNumberRegExpsAndMappings {
    fn initialize_regexp_mappings(&mut self) {
        self.mobile_token_mappings.insert(54, '9');
        self.geo_mobile_countries_without_mobile_area_codes
            .insert(86); // China
        self.countries_without_national_prefix_with_area_codes
            .insert(52); // Mexico

        self.geo_mobile_countries.insert(52); // Mexico
        self.geo_mobile_countries.insert(54); // Argentina
        self.geo_mobile_countries.insert(55); // Brazil
        self.geo_mobile_countries.insert(62); // Indonesia
        self.geo_mobile_countries.insert(86); // China 

        for (ch, digit) in [
            ('A', '2'),
            ('B', '2'),
            ('C', '2'),
            ('D', '3'),
            ('E', '3'),
            ('F', '3'),
            ('G', '4'),
            ('H', '4'),
            ('I', '4'),
            ('J', '5'),
            ('K', '5'),
            ('L', '5'),
            ('M', '6'),
            ('N', '6'),
            ('O', '6'),
            ('P', '7'),
            ('Q', '7'),
            ('R', '7'),
            ('S', '7'),
            ('T', '8'),
            ('U', '8'),
            ('V', '8'),
            ('W', '9'),
            ('X', '9'),
            ('Y', '9'),
            ('Z', '9'),
        ] {
            self.alpha_mappings.insert(ch, digit);
            self.alpha_phone_mappings.insert(ch, digit);
            self.all_plus_number_grouping_symbols
                .insert(ch.to_ascii_lowercase(), ch);
            self.all_plus_number_grouping_symbols.insert(ch, ch);
        }

        for d in '0'..='9' {
            self.alpha_phone_mappings.insert(d, d);
            self.diallable_char_mappings.insert(d, d);
            self.all_plus_number_grouping_symbols.insert(d, d);
        }

        self.diallable_char_mappings.insert('+', '+');
        self.diallable_char_mappings.insert('*', '*');
        self.diallable_char_mappings.insert('#', '#');

        for (ch, target) in [
            ('-', '-'),
            ('\u{FF0D}', '-'),
            ('\u{2010}', '-'),
            ('\u{2011}', '-'),
            ('\u{2012}', '-'),
            ('\u{2013}', '-'),
            ('\u{2014}', '-'),
            ('\u{2015}', '-'),
            ('\u{2212}', '-'),
            ('/', '/'),
            ('\u{FF0F}', '/'),
            (' ', ' '),
            ('\u{3000}', ' '),
            ('\u{2060}', ' '),
            ('.', '.'),
            ('\u{FF0E}', '.'),
        ] {
            self.all_plus_number_grouping_symbols.insert(ch, target);
        }
    }

    pub(crate) fn new() -> Self {
        let alphanum = fast_cat::concat_str!(VALID_ALPHA_INCL_UPPERCASE, DIGITS);
        let extn_patterns_for_parsing = create_extn_pattern(true);
        let valid_phone_number = format!(
            "[{}]*(?:[{}{}]*[{}]){{3,}}[{}{}{}{}]*|[{}]{{{}}}",
            PLUS_CHARS,
            VALID_PUNCTUATION,
            STAR_SIGN,
            DIGITS,
            VALID_PUNCTUATION,
            STAR_SIGN,
            VALID_ALPHA,
            DIGITS,
            DIGITS,
            MIN_LENGTH_FOR_NSN,
        );

        let rfc3966_phone_digit = format!("([{}]|{})", DIGITS, RFC3966_VISUAL_SEPARATOR);
        let rfc3966_domainlabel = format!("[{}]+((\\-)*[{}])*", alphanum, alphanum);
        let rfc3966_toplabel = format!("[{}]+((\\-)*[{}])*", VALID_ALPHA_INCL_UPPERCASE, alphanum);
        let catch_all_formatting_regex = RegexTriplets::new(Some("^(?:(\\d+)(.*))$".to_string()));
        catch_all_formatting_regex.original().unwrap();

        let mut instance = Self {
            diallable_char_mappings: FxHashMap::with_capacity_and_hasher(13, Default::default()),
            alpha_mappings: FxHashMap::with_capacity_and_hasher(26, Default::default()),
            alpha_phone_mappings: FxHashMap::with_capacity_and_hasher(36, Default::default()),
            all_plus_number_grouping_symbols: FxHashMap::with_capacity_and_hasher(
                80,
                Default::default(),
            ),
            mobile_token_mappings: FxHashMap::with_capacity_and_hasher(1, Default::default()),
            countries_without_national_prefix_with_area_codes: FxHashSet::with_capacity_and_hasher(
                1,
                Default::default(),
            ),
            geo_mobile_countries: FxHashSet::with_capacity_and_hasher(5, Default::default()),
            geo_mobile_countries_without_mobile_area_codes: FxHashSet::with_capacity_and_hasher(
                1,
                Default::default(),
            ),

            single_international_prefix_fullmatch: Regex::new(
                "^(?:[\\d]+(?:[~\u{2053}\u{223C}\u{FF5E}][\\d]+)?)$",
            )
            .unwrap(),
            capturing_ascii_digits_pattern: Regex::new("(\\d+)").unwrap(),
            capture_up_to_second_number_start_pattern: Regex::new(
                CAPTURE_UP_TO_SECOND_NUMBER_START,
            )
            .unwrap(),
            separator_pattern_anchor_start: Regex::new(&format!("^[{}]+", VALID_PUNCTUATION))
                .unwrap(),
            separator_pattern: Regex::new(&format!("[{}]+", VALID_PUNCTUATION)).unwrap(),
            extn_pattern: Regex::new(&format!("(?i)(?:{})$", &extn_patterns_for_parsing)).unwrap(),
            valid_phone_number_pattern_fullmatch: Regex::new(&format!(
                "(?i)^(?:{})(?:{})?$",
                &valid_phone_number, &extn_patterns_for_parsing
            ))
            .unwrap(),
            valid_alpha_phone_pattern_fullmatch: Regex::new("^(?:.*?[A-Za-z]){3}.*$").unwrap(),
            first_group_capturing_pattern: Regex::new("(\\$\\d)").unwrap(),
            carrier_code_pattern: Regex::new("\\$CC").unwrap(),
            plus_chars_pattern_start: Regex::new(&format!("^[{}]+", &PLUS_CHARS)).unwrap(),
            rfc3966_global_number_digits_pattern_fullmatch: Regex::new(&format!(
                "^\\{}{}*[{}]{}*$",
                PLUS_SIGN, &rfc3966_phone_digit, DIGITS, rfc3966_phone_digit
            ))
            .unwrap(),
            rfc3966_domainname_pattern_fullmatch: Regex::new(&format!(
                "^({}\\.)*{}\\.?$",
                rfc3966_domainlabel, rfc3966_toplabel
            ))
            .unwrap(),
            is_format_eligible_as_you_type_formatting_regex_fullmatch: Regex::new(&format!(
                "^(?:[{}]*\\$1[{}]*(\\$\\d[{}]*)*)$",
                VALID_PUNCTUATION, VALID_PUNCTUATION, VALID_PUNCTUATION
            ))
            .unwrap(),
            formatting_rule_has_first_group_only_regex_fullmatch: Regex::new("^\\(?\\$1\\)?$")
                .unwrap(),
            catch_all_formatting_regex,
        };

        instance.initialize_regexp_mappings();
        instance
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_regexps_are_compiling() {
        super::PhoneNumberRegExpsAndMappings::new();
    }
}
