use crate::regexp::Regex;

use crate::phonenumberutil::{
    helper_constants::{
        DIGITS, MAX_LENGTH_COUNTRY_CODE, MAX_LENGTH_FOR_NSN, PLUS_CHARS, SEPARATORS,
        VALID_PUNCTUATION,
    },
    helper_functions::create_extn_pattern,
};

#[derive(Debug, Clone)]
pub struct MatcherRegex {
    pub pattern: Regex,
    pub pub_pages: Regex,
    pub slash_separated_dates: Regex,
    pub time_stamps: Regex,
    pub time_stamps_suffix: Regex,
    pub matching_brackets_full_match: Regex,
    pub inner_matches: Vec<Regex>,
    pub lead_class: Regex,
}

/// Returns a regular expression quantifier with an upper and lower limit.
fn limit(lower: usize, upper: usize) -> String {
    debug_assert!(
        upper > 0 && upper >= lower,
        "invalid limit bounds: lower={lower}, upper={upper}"
    );
    format!("{{{lower},{upper}}}")
}

impl MatcherRegex {
    pub fn new() -> Self {
        let opening_parens = "(\\[\u{FF08}\u{FF3B}";
        let closing_parens = ")\\]\u{FF09}\u{FF3D}";
        let non_parens = format!("[^{opening_parens}{closing_parens}]");

        let bracket_pair_limit = limit(0, 3);

        let matching_brackets_full_match = Regex::new(&format!(
            "^(?:(?:[{opening_parens}])?(?:{non_parens}+[{closing_parens}])?{non_parens}+\
             (?:[{opening_parens}]{non_parens}+[{closing_parens}]){bracket_pair_limit}{non_parens}*)$"
        )).unwrap();

        let lead_limit = limit(0, 2);
        let punctuation_limit = limit(0, 4);
        let digit_block_limit = MAX_LENGTH_FOR_NSN + MAX_LENGTH_COUNTRY_CODE;
        let block_limit = limit(0, digit_block_limit);

        let punctuation = format!("[{}]{punctuation_limit}", VALID_PUNCTUATION);
        let digit_sequence = format!("[{}]{}", DIGITS, limit(1, digit_block_limit));

        let lead_class_chars = format!("{opening_parens}{}", PLUS_CHARS);
        let lead_class_str = format!("[{lead_class_chars}]");
        let lead_class = Regex::new(&lead_class_str).unwrap();

        let pattern = Regex::new(&format!(
            "(?:{lead_class_str}{punctuation}){lead_limit}\
             {digit_sequence}(?:{punctuation}{digit_sequence}){block_limit}\
             (?:{})?",
            create_extn_pattern(false),
        ))
        .unwrap();

        let inner_matches = vec![
            Regex::new("/+(.*)").unwrap(),
            Regex::new("(\\([^(]*)").unwrap(),
            Regex::new(&format!(
                "(?:[{}]-|-[{}])[{}]*(.+)",
                SEPARATORS, SEPARATORS, SEPARATORS
            ))
            .unwrap(),
            Regex::new(&format!("[\u{2012}-\u{2015}\u{FF0D}][{}]*(.+)", SEPARATORS)).unwrap(),
            Regex::new(&format!("\\.+[{}]*([^.]+)", SEPARATORS)).unwrap(),
            Regex::new(&format!("[{}]+([{}]+)", SEPARATORS, SEPARATORS)).unwrap(),
        ];

        Self {
            pattern,
            pub_pages: Regex::new("\\d{1,5}-+\\d{1,5}\\s{0,4}\\(\\d{1,4}").unwrap(),
            slash_separated_dates: Regex::new(
                "(?:(?:[0-3]?\\d/[01]?\\d)|(?:[01]?\\d/[0-3]?\\d))/(?:[12]\\d)?\\d{2}",
            )
            .unwrap(),
            time_stamps: Regex::new("[12]\\d{3}[-/]?[01]\\d[-/]?[0-3]\\d +[0-2]\\d$").unwrap(),
            time_stamps_suffix: Regex::new(":[0-5]\\d").unwrap(),
            matching_brackets_full_match,
            inner_matches,
            lead_class,
        }
    }
}
