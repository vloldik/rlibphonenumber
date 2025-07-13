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


use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::{phonenumberutil::{helper_constants::{
    CAPTURE_UP_TO_SECOND_NUMBER_START, DIGITS, MIN_LENGTH_FOR_NSN, PLUS_CHARS, 
    PLUS_SIGN, RFC3966_VISUAL_SEPARATOR, STAR_SIGN, VALID_ALPHA, VALID_ALPHA_INCL_UPPERCASE, 
    VALID_PUNCTUATION
}, helper_functions::create_extn_pattern}, regexp_cache::RegexCache};

#[allow(unused)]
pub(super) struct PhoneNumberRegExpsAndMappings {
    /// Regular expression of viable phone numbers. This is location independent.
    /// Checks we have at least three leading digits, and only valid punctuation,
    /// alpha characters and digits in the phone number. Does not include extension
    /// data. The symbol 'x' is allowed here as valid punctuation since it is often
    /// used as a placeholder for carrier codes, for example in Brazilian phone
    /// numbers. We also allow multiple plus-signs at the start.
    /// 
    /// Corresponds to the following:
    /// `[digits]{minLengthNsn}|
    /// plus_sign*(([punctuation]|[star])*[digits]){3,}
    /// ([punctuation]|[star]|[digits]|[alpha])*`
    ///
    /// The first reg-ex is to allow short numbers (two digits long) to be parsed
    /// if they are entered as "15" etc, but only if there is no punctuation in
    /// them. The second expression restricts the number of digits to three or
    /// more, but then allows them to be in international form, and to have
    /// alpha-characters and punctuation.
    valid_phone_number: String,

    /// Regexp of all possible ways to write extensions, for use when parsing. This
    /// will be run as a case-insensitive regexp match. Wide character versions are
    /// also provided after each ASCII version.
    /// For parsing, we are slightly more lenient in our interpretation than for
    /// matching. Here we allow "comma" and "semicolon" as possible extension
    /// indicators. When matching, these are hardly ever used to indicate this.
    extn_patterns_for_parsing: String,

    /// Regular expressions of different parts of the phone-context parameter,
    /// following the syntax defined in RFC3966.
    rfc3966_phone_digit: String,
    alphanum: String,
    rfc3966_domainlabel: String,
    rfc3966_toplabel: String,

    pub regexp_cache: RegexCache,

    /// A map that contains characters that are essential when dialling. That means
    /// any of the characters in this map must not be removed from a number when
    /// dialing, otherwise the call will not reach the intended destination.
    pub diallable_char_mappings: HashMap<char, char>,
    /// These mappings map a character (key) to a specific digit that should
    /// replace it for normalization purposes.
    pub alpha_mappings: HashMap<char, char>,
    /// For performance reasons, store a map of combining alpha_mappings with ASCII
    /// digits.
    pub alpha_phone_mappings: HashMap<char, char>,

    /// Separate map of all symbols that we wish to retain when formatting alpha
    /// numbers. This includes digits, ascii letters and number grouping symbols
    /// such as "-" and " ".
    pub all_plus_number_grouping_symbols: HashMap<char, char>,

    /// Map of country calling codes that use a mobile token before the area code.
    /// One example of when this is relevant is when determining the length of the
    /// national destination code, which should be the length of the area code plus
    /// the length of the mobile token.
    pub mobile_token_mappings: HashMap<i32, char>,

    /// Set of country codes that doesn't have national prefix, but it has area
    /// codes.
    pub countries_without_national_prefix_with_area_codes: HashSet<i32>,

    /// Set of country codes that have geographically assigned mobile numbers (see
    /// geo_mobile_countries_ below) which are not based on *area codes*. For
    /// example, in China mobile numbers start with a carrier indicator, and beyond
    /// that are geographically assigned: this carrier indicator is not considered
    /// to be an area code.
    pub geo_mobile_countries_without_mobile_area_codes: HashSet<i32>,

    /// Set of country calling codes that have geographically assigned mobile
    /// numbers. This may not be complete; we add calling codes case by case, as we
    /// find geographical mobile numbers or hear from user reports.
    pub geo_mobile_countries: HashSet<i32>,

    /// Pattern that makes it easy to distinguish whether a region has a single
    /// international dialing prefix or not. If a region has a single international
    /// prefix (e.g. 011 in USA), it will be represented as a string that contains
    /// a sequence of ASCII digits, and possibly a tilde, which signals waiting for
    /// the tone. If there are multiple available international prefixes in a
    /// region, they will be represented as a regex string that always contains one
    /// or more characters that are not ASCII digits or a tilde.
    pub single_international_prefix: Regex,

    pub digits_pattern: Regex,
    pub capturing_digit_pattern: Regex,
    pub capturing_ascii_digits_pattern: Regex,

    /// Regular expression of acceptable characters that may start a phone number
    /// for the purposes of parsing. This allows us to strip away meaningless
    /// prefixes to phone numbers that may be mistakenly given to us. This consists
    /// of digits, the plus symbol and arabic-indic digits. This does not contain
    /// alpha characters, although they may be used later in the number. It also
    /// does not include other punctuation, as this will be stripped later during
    /// parsing and is of no information value when parsing a number. The string
    /// starting with this valid character is captured.
    /// This corresponds to VALID_START_CHAR in the java version.
    pub valid_start_char_pattern: Regex,

    /// Regular expression of valid characters before a marker that might indicate
    /// a second number.
    pub capture_up_to_second_number_start_pattern: Regex,

    /// Regular expression of trailing characters that we want to remove. We remove
    /// all characters that are not alpha or numerical characters. The hash
    /// character is retained here, as it may signify the previous block was an
    /// extension. Note the capturing block at the start to capture the rest of the
    /// number if this was a match.
    /// This corresponds to UNWANTED_END_CHAR_PATTERN in the java version.
    pub unwanted_end_char_pattern: Regex,

    /// Regular expression of groups of valid punctuation characters.
    pub separator_pattern: Regex,

    /// Regexp of all possible ways to write extensions, for use when finding phone
    /// numbers in text. This will be run as a case-insensitive regexp match. Wide
    /// character versions are also provided after each ASCII version.
    pub extn_patterns_for_matching: String,

    /// Regexp of all known extension prefixes used by different regions followed
    /// by 1 or more valid digits, for use when parsing.
    pub extn_pattern: Regex,

    /// We append optionally the extension pattern to the end here, as a valid
    /// phone number may have an extension prefix appended, followed by 1 or more
    /// digits.
    pub valid_phone_number_pattern: Regex,

    /// We use this pattern to check if the phone number has at least three letters
    /// in it - if so, then we treat it as a number where some phone-number digits
    /// are represented by letters.
    pub valid_alpha_phone_pattern: Regex,

    pub first_group_capturing_pattern: Regex,

    pub carrier_code_pattern: Regex,

    pub plus_chars_pattern: Regex,

    /// Regular expression of valid global-number-digits for the phone-context
    /// parameter, following the syntax defined in RFC3966.
    pub rfc3966_global_number_digits_pattern: Regex,

    /// Regular expression of valid domainname for the phone-context parameter,
    /// following the syntax defined in RFC3966.
    pub rfc3966_domainname_pattern: Regex,

    /// *Rust note*: It's for some reason calculated inside function in C++,
    /// so, we move it here
    /// 
    /// A pattern that is used to determine if a numberFormat under
    /// availableFormats is eligible to be used by the AYTF. It is eligible when
    /// the format element under numberFormat contains groups of the dollar sign
    /// followed by a single digit, separated by valid phone number punctuation.
    /// This prevents invalid punctuation (such as the star sign in Israeli star
    /// numbers) getting into the output of the AYTF. 
    pub is_format_eligible_as_you_type_formatting_regex: Regex,

    /// Added for function `formatting_rule_has_first_group_only`
    /// A pattern that is used to determine if the national prefix formatting rule
    /// has the first group only, i.e., does not start with the national prefix.
    /// Note that the pattern explicitly allows for unbalanced parentheses.
    pub formatting_rule_has_first_group_only_regex: Regex
}

impl PhoneNumberRegExpsAndMappings {
    fn initialize_regexp_mappings(&mut self) {
        self.mobile_token_mappings.insert(54, '9');

        self.geo_mobile_countries_without_mobile_area_codes.insert(86);  // China

        self.countries_without_national_prefix_with_area_codes.insert(52);  // Mexico

        self.geo_mobile_countries.insert(52);  // Mexico
        self.geo_mobile_countries.insert(54);  // Argentina
        self.geo_mobile_countries.insert(55);  // Brazil
        self.geo_mobile_countries.insert(62);  // Indonesia: some prefixes only (fixed CMDA wireless)
        self.geo_mobile_countries.extend(&self.geo_mobile_countries_without_mobile_area_codes);

        // Simple ASCII digits map used to populate ALPHA_PHONE_MAPPINGS and
        // ALL_PLUS_NUMBER_GROUPING_SYMBOLS.
        let mut ascii_digit_mappings = HashMap::with_capacity(10);
        for d in '0'..='9' {
            ascii_digit_mappings.insert(d, d);
        }

        let mut alpha_map = HashMap::with_capacity(40);
        alpha_map.insert('A', '2');
        alpha_map.insert('B', '2');
        alpha_map.insert('C', '2');
        alpha_map.insert('D', '3');
        alpha_map.insert('E', '3');
        alpha_map.insert('F', '3');
        alpha_map.insert('G', '4');
        alpha_map.insert('H', '4');
        alpha_map.insert('I', '4');
        alpha_map.insert('J', '5');
        alpha_map.insert('K', '5');
        alpha_map.insert('L', '5');
        alpha_map.insert('M', '6');
        alpha_map.insert('N', '6');
        alpha_map.insert('O', '6');
        alpha_map.insert('P', '7');
        alpha_map.insert('Q', '7');
        alpha_map.insert('R', '7');
        alpha_map.insert('S', '7');
        alpha_map.insert('T', '8');
        alpha_map.insert('U', '8');
        alpha_map.insert('V', '8');
        alpha_map.insert('W', '9');
        alpha_map.insert('X', '9');
        alpha_map.insert('Y', '9');
        alpha_map.insert('Z', '9');
        // IMPORTANT: only uppercase letters like in Java version

        self.alpha_mappings = alpha_map;

        let mut combined_map = HashMap::with_capacity(100);
        combined_map.extend(self.alpha_mappings.iter());
        combined_map.extend(ascii_digit_mappings.iter());
        self.alpha_phone_mappings = combined_map;

        let mut dilatable_char_map = HashMap::new();
        dilatable_char_map.extend(ascii_digit_mappings.iter());
        dilatable_char_map.insert('+', '+');
        dilatable_char_map.insert('*', '*');
        dilatable_char_map.insert('#', '#');
        self.diallable_char_mappings = dilatable_char_map;

        let mut all_plus_number_groupings = HashMap::new();
        // insert (lower letter -> upper letter) and (upper letter -> upper letter) mappings.
        for c in self.alpha_mappings.keys() {
            all_plus_number_groupings.insert(c.to_ascii_lowercase(), *c);
            all_plus_number_groupings.insert(*c, *c);
        }
        all_plus_number_groupings.extend(ascii_digit_mappings.iter());
        // insert grouping symbols.
        all_plus_number_groupings.insert('-', '-');
        all_plus_number_groupings.insert('\u{FF0D}', '-');
        all_plus_number_groupings.insert('\u{2010}', '-');
        all_plus_number_groupings.insert('\u{2011}', '-');
        all_plus_number_groupings.insert('\u{2012}', '-');
        all_plus_number_groupings.insert('\u{2013}', '-');
        all_plus_number_groupings.insert('\u{2014}', '-');
        all_plus_number_groupings.insert('\u{2015}', '-');
        all_plus_number_groupings.insert('\u{2212}', '-');
        all_plus_number_groupings.insert('/', '/');
        all_plus_number_groupings.insert('\u{FF0F}', '/');
        all_plus_number_groupings.insert(' ', ' ');
        all_plus_number_groupings.insert('\u{3000}', ' ');
        all_plus_number_groupings.insert('\u{2060}', ' ');
        all_plus_number_groupings.insert('.', '.');
        all_plus_number_groupings.insert('\u{FF0E}', '.');
        self.all_plus_number_grouping_symbols = all_plus_number_groupings;
    }

    pub fn new() -> Self {
        let alphanum = fast_cat::concat_str!(VALID_ALPHA_INCL_UPPERCASE, DIGITS);
        let extn_patterns_for_parsing = create_extn_pattern(true);
        let valid_phone_number = format!(
                // moved 2-digits pattern to an end for match full number first
                "[{}]*(?:[{}{}]*{}){{3,}}[{}{}{}{}]*|{}{{{}}}",
                PLUS_CHARS,
                VALID_PUNCTUATION, STAR_SIGN, DIGITS,
                VALID_PUNCTUATION, STAR_SIGN, DIGITS, VALID_ALPHA,
                DIGITS, MIN_LENGTH_FOR_NSN, 
            );

        let rfc3966_phone_digit = format!("({}|{})", DIGITS, RFC3966_VISUAL_SEPARATOR);
        let rfc3966_domainlabel = format!("[{}]+((\\-)*[{}])*", alphanum, alphanum);
        let rfc3966_toplabel = format!("[{}]+((\\-)*[{}])*", VALID_ALPHA_INCL_UPPERCASE, alphanum);

        let mut instance = Self{
            // it'll be initialized only once, so we can use slow format!
            valid_phone_number: valid_phone_number.clone(),
            extn_patterns_for_parsing: extn_patterns_for_parsing.clone(),
            rfc3966_phone_digit: rfc3966_phone_digit.clone(),
            alphanum: alphanum,
            rfc3966_domainlabel: rfc3966_domainlabel.clone(),
            rfc3966_toplabel: rfc3966_toplabel.clone(),
            regexp_cache: RegexCache::with_capacity(128),
            diallable_char_mappings: Default::default(),
            alpha_mappings: Default::default(),
            alpha_phone_mappings: Default::default(),
            all_plus_number_grouping_symbols: Default::default(),
            mobile_token_mappings: Default::default(),
            countries_without_national_prefix_with_area_codes: Default::default(),
            geo_mobile_countries: Default::default(),
            geo_mobile_countries_without_mobile_area_codes: Default::default(),
            single_international_prefix: Regex::new("[\\d]+(?:[~\u{2053}\u{223C}\u{FF5E}][\\d]+)?").unwrap(),
            digits_pattern: Regex::new(&format!("[{}]*", DIGITS)).unwrap(),
            capturing_digit_pattern: Regex::new(&format!("([{}])", DIGITS)).unwrap(),
            capturing_ascii_digits_pattern: Regex::new("(\\d+)").unwrap(),
            valid_start_char_pattern: Regex::new(&format!("[{}{}]", PLUS_CHARS, DIGITS)).unwrap(),
            capture_up_to_second_number_start_pattern: Regex::new(CAPTURE_UP_TO_SECOND_NUMBER_START).unwrap(),
            unwanted_end_char_pattern: Regex::new("[^\\p{N}\\p{L}#]").unwrap(),
            separator_pattern: Regex::new(&format!("[{}]+", VALID_PUNCTUATION)).unwrap(),
            extn_patterns_for_matching: create_extn_pattern(false),
            extn_pattern: Regex::new(&format!("(?i)(?:{})$", &extn_patterns_for_parsing)).unwrap(),
            valid_phone_number_pattern: Regex::new(&format!("(?i)^(?:{})(?:{})?$", 
                &valid_phone_number,
                &extn_patterns_for_parsing
            )).unwrap(),
            // from java
            valid_alpha_phone_pattern: Regex::new("(?:.*?[A-Za-z]){3}.*").unwrap(),
            // The first_group_capturing_pattern was originally set to $1 but there
            // are some countries for which the first group is not used in the
            // national pattern (e.g. Argentina) so the $1 group does not match
            // correctly. Therefore, we use \d, so that the first group actually
            // used in the pattern will be matched.
            first_group_capturing_pattern: Regex::new("(\\$\\d)").unwrap(),
            carrier_code_pattern: Regex::new("\\$CC").unwrap(),
            plus_chars_pattern: Regex::new(&format!("[{}]+", &PLUS_CHARS)).unwrap(),
            rfc3966_global_number_digits_pattern: Regex::new(
                &format!("^\\{}{}*{}{}*$", PLUS_SIGN, &rfc3966_phone_digit, DIGITS, rfc3966_phone_digit)
            ).unwrap(),
            rfc3966_domainname_pattern: Regex::new(
                &format!("^({}\\.)*{}\\.?$", rfc3966_domainlabel, rfc3966_toplabel)
            ).unwrap(),
            is_format_eligible_as_you_type_formatting_regex: Regex::new(
                &format!("[{}]*\\$1[{}]*(\\$\\d[{}]*)*",VALID_PUNCTUATION, VALID_PUNCTUATION, VALID_PUNCTUATION)
            ).unwrap(),
            formatting_rule_has_first_group_only_regex: Regex::new("\\(?\\$1\\)?").unwrap()
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