#![allow(clippy::field_reassign_with_default)]

#[cfg(test)]
mod tests {
    use crate::{
        CountryCodeSource, PhoneNumber, Region,
        phonenumber_matcher::{
            Leniency, PhoneNumberMatch, PhoneNumberMatcher, PhoneNumberMatcherInternal,
        },
        phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
        tests::common::{get_phone_matcher_factory, get_phone_util},
    };

    fn phone_util() -> &'static PhoneNumberUtilInternal {
        get_phone_util()
    }

    fn find_numbers(
        text: &str,
        region: Option<Region>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal> {
        get_phone_matcher_factory().create_matcher(text, Leniency::Valid, u64::MAX, region)
    }

    fn find_numbers_for_leniency<'a>(
        text: &'a str,
        region: Option<Region>,
        leniency: Leniency,
    ) -> impl Iterator<Item = PhoneNumberMatch<'a>> {
        get_phone_matcher_factory().create_matcher(text, leniency, u64::MAX, region)
    }

    fn has_no_matches<'a, I>(mut iterator: I) -> bool
    where
        I: Iterator<Item = PhoneNumberMatch<'a>>,
    {
        iterator.next().is_none()
    }

    #[test]
    fn test_contains_more_than_one_slash_in_national_number() {
        let matcher = find_numbers("dummy", None);

        // A date should return true.
        let mut number = PhoneNumber::default();
        number.country_code = 1;
        number.set_country_code_source(CountryCodeSource::FromDefaultCountry);
        let mut candidate = "1/05/2013";
        assert!(
            matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        // Here, the country code source thinks it started with a country calling code, but this is not
        // the same as the part before the slash, so it's still true.
        let mut number = PhoneNumber::default();
        number.country_code = 274;
        number.set_country_code_source(CountryCodeSource::FromNumberWithoutPlusSign);
        candidate = "27/4/2013";
        assert!(
            matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        // Now it should be false, because the first slash is after the country calling code.
        let mut number = PhoneNumber::default();
        number.country_code = 49;
        number.set_country_code_source(CountryCodeSource::FromNumberWithPlusSign);
        candidate = "49/69/2013";
        assert!(
            !matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        let mut number = PhoneNumber::default();
        number.country_code = 49;
        number.set_country_code_source(CountryCodeSource::FromNumberWithoutPlusSign);
        candidate = "+49/69/2013";
        assert!(
            !matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        candidate = "+ 49/69/2013";
        assert!(
            !matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        candidate = "+ 49/69/20/13";
        assert!(
            matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );

        // Here, the first group is not assumed to be the country calling code, even though it is the
        // same as it, so this should return true.
        let mut number = PhoneNumber::default();
        number.country_code = 49;
        number.set_country_code_source(CountryCodeSource::FromDefaultCountry);
        candidate = "49/69/2013";
        assert!(
            matcher
                .inner
                .contains_more_than_one_slash_in_national_number(&number, candidate)
        );
    }

    #[test]
    fn test_find_national_number() {
        do_test_find_in_context("033316005", Some(Region::NZ));
        do_test_find_in_context("03-331 6005", Some(Region::NZ));
        do_test_find_in_context("03 331 6005", Some(Region::NZ));
        do_test_find_in_context("0064 3 331 6005", Some(Region::NZ));
        do_test_find_in_context("01164 3 331 6005", Some(Region::US));
        do_test_find_in_context("+64 3 331 6005", Some(Region::US));
        do_test_find_in_context("64(0)64123456", Some(Region::NZ));
        do_test_find_in_context("0123/456789", Some(Region::PL));
        do_test_find_in_context("123-456-7890", Some(Region::US));
    }

    #[test]
    fn test_find_with_international_prefixes() {
        do_test_find_in_context("+1 (650) 333-6000", Some(Region::NZ));
        do_test_find_in_context("1-650-333-6000", Some(Region::US));
        do_test_find_in_context("0011-650-333-6000", Some(Region::SG));
        do_test_find_in_context("0081-650-333-6000", Some(Region::SG));
        do_test_find_in_context("0191-650-333-6000", Some(Region::SG));
        do_test_find_in_context("0~01-650-333-6000", Some(Region::PL));
        do_test_find_in_context("++1 (650) 333-6000", Some(Region::PL));
        do_test_find_in_context("\u{FF0B}1 (650) 333-6000", Some(Region::SG));
        do_test_find_in_context(
            "\u{FF0B}\u{FF11}\u{3000}\u{FF08}\u{FF16}\u{FF15}\u{FF10}\u{FF09}\u{3000}\u{FF13}\u{FF13}\u{FF13}\u{FF0D}\u{FF16}\u{FF10}\u{FF10}\u{FF10}",
            Some(Region::SG),
        );
    }

    #[test]
    fn test_find_with_leading_zero() {
        do_test_find_in_context("+39 02-36618 300", Some(Region::NZ));
        do_test_find_in_context("02-36618 300", Some(Region::IT));
        do_test_find_in_context("312 345 678", Some(Region::IT));
    }

    #[test]
    fn test_find_national_number_argentina() {
        do_test_find_in_context("+54 9 343 555 1212", Some(Region::AR));
        do_test_find_in_context("0343 15 555 1212", Some(Region::AR));

        do_test_find_in_context("+54 9 3715 65 4320", Some(Region::AR));
        do_test_find_in_context("03715 15 65 4320", Some(Region::AR));

        do_test_find_in_context("+54 11 3797 0000", Some(Region::AR));
        do_test_find_in_context("011 3797 0000", Some(Region::AR));

        do_test_find_in_context("+54 3715 65 4321", Some(Region::AR));
        do_test_find_in_context("03715 65 4321", Some(Region::AR));

        do_test_find_in_context("+54 23 1234 0000", Some(Region::AR));
        do_test_find_in_context("023 1234 0000", Some(Region::AR));
    }

    #[test]
    fn test_find_with_x_in_number() {
        do_test_find_in_context("(0xx) 123456789", Some(Region::AR));
        do_test_find_in_context("(0xx) 123456789 x 1234", Some(Region::AR));
        do_test_find_in_context("011xx5481429712", Some(Region::US));
    }

    #[test]
    fn test_find_numbers_mexico() {
        do_test_find_in_context("+52 (449)978-0001", Some(Region::MX));
        do_test_find_in_context("01 (449)978-0001", Some(Region::MX));
        do_test_find_in_context("(449)978-0001", Some(Region::MX));

        do_test_find_in_context("+52 1 33 1234-5678", Some(Region::MX));
        do_test_find_in_context("044 (33) 1234-5678", Some(Region::MX));
        do_test_find_in_context("045 33 1234-5678", Some(Region::MX));
    }

    #[test]
    fn test_find_numbers_with_plus_with_no_region() {
        do_test_find_in_context("+64 3 331 6005", None);
        do_test_find_in_context("+64 3 331 6005", None);
    }

    #[test]
    fn test_find_extensions() {
        do_test_find_in_context("03 331 6005 ext 3456", Some(Region::NZ));
        do_test_find_in_context("03-3316005x3456", Some(Region::NZ));
        do_test_find_in_context("03-3316005 int.3456", Some(Region::NZ));
        do_test_find_in_context("03 3316005 #3456", Some(Region::NZ));
        do_test_find_in_context("0~0 1800 7493 524", Some(Region::PL));
        do_test_find_in_context("(1800) 7493.524", Some(Region::US));
        do_test_find_in_context("0~0 1800 7493 524 ~1234", Some(Region::PL));

        do_test_find_in_context("+44 2034567890x456", Some(Region::NZ));
        do_test_find_in_context("+44 2034567890x456", Some(Region::GB));
        do_test_find_in_context("+44 2034567890 x456", Some(Region::GB));
        do_test_find_in_context("+44 2034567890 X456", Some(Region::GB));
        do_test_find_in_context("+44 2034567890 X 456", Some(Region::GB));
        do_test_find_in_context("+44 2034567890 X  456", Some(Region::GB));
        do_test_find_in_context("+44 2034567890  X 456", Some(Region::GB));

        do_test_find_in_context("(800) 901-3355 x 7246433", Some(Region::US));
        do_test_find_in_context("(800) 901-3355 , ext 7246433", Some(Region::US));
        do_test_find_in_context("(800) 901-3355 ,extension 7246433", Some(Region::US));
        do_test_find_in_context("(800) 901-3355 ,x 7246433", Some(Region::US));
        do_test_find_in_context("(800) 901-3355 ext: 7246433", Some(Region::US));
    }

    #[test]
    fn test_find_interspersed_with_space() {
        do_test_find_in_context("0 3   3 3 1   6 0 0 5", Some(Region::NZ));
    }

    #[test]
    fn test_intermediate_parse_positions() {
        let text = "Call 033316005  or 032316005!";
        for i in 0..=5 {
            assert_equal_range(text, i, 5, 14);
        }
        assert_equal_range(text, 6, 6, 14);
        assert_equal_range(text, 7, 7, 14);
        for i in 8..=19 {
            assert_equal_range(text, i, 19, 28);
        }
    }

    #[test]
    fn test_four_matches_in_a_row() {
        let number1 = "415-666-7777";
        let number2 = "800-443-1223";
        let number3 = "212-443-1223";
        let number4 = "650-443-1223";
        let text = format!("{} - {} - {} - {}", number1, number2, number3, number4);

        let mut iterator = find_numbers(&text, Some(Region::US));

        assert_match_properties(iterator.next(), &text, number1, Some(Region::US));
        assert_match_properties(iterator.next(), &text, number2, Some(Region::US));
        assert_match_properties(iterator.next(), &text, number3, Some(Region::US));
        assert_match_properties(iterator.next(), &text, number4, Some(Region::US));
    }

    #[test]
    fn test_matches_found_with_multiple_spaces() {
        let number1 = "(415) 666-7777";
        let number2 = "(800) 443-1223";
        let text = format!("{} {}", number1, number2);

        let mut iterator = find_numbers(&text, Some(Region::US));
        assert_match_properties(iterator.next(), &text, number1, Some(Region::US));
        assert_match_properties(iterator.next(), &text, number2, Some(Region::US));
    }

    #[test]
    fn test_match_with_surrounding_zipcodes() {
        let number = "415-666-7777";
        let zip_preceding = format!("My address is CA 34215 - {} is my number.", number);

        let mut iterator = find_numbers(&zip_preceding, Some(Region::US));
        assert_match_properties(iterator.next(), &zip_preceding, number, Some(Region::US));

        let number_with_spaces = "(415) 666 7777";
        let zip_following = format!("My number is {}. 34215 is my zip-code.", number_with_spaces);

        let mut iterator = find_numbers(&zip_following, Some(Region::US));
        assert_match_properties(
            iterator.next(),
            &zip_following,
            number_with_spaces,
            Some(Region::US),
        );
    }

    #[test]
    fn test_find_with_x_delim_and_ext() {
        let str =
            "+7)7778777.777X88665X3- URq+1-800-123-32-17X3222222222222222222222222222222222DXXXX31";

        let found = find_numbers(str, None).next().unwrap();

        assert!(
            found.raw_string == "+1-800-123-32-17X322222222" && found.number.country_code == 1,
            "Number with X ext should be found"
        )
    }

    #[test]
    fn test_is_latin_letter() {
        type DummyMatcher<'a> = PhoneNumberMatcherInternal<
            'a,
            crate::phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
            &'a crate::phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal,
        >;

        assert!(DummyMatcher::is_latin_letter('c'));
        assert!(DummyMatcher::is_latin_letter('C'));
        assert!(DummyMatcher::is_latin_letter('\u{00C9}'));
        assert!(DummyMatcher::is_latin_letter('\u{0301}'));
        assert!(!DummyMatcher::is_latin_letter(':'));
        assert!(!DummyMatcher::is_latin_letter('5'));
        assert!(!DummyMatcher::is_latin_letter('-'));
        assert!(!DummyMatcher::is_latin_letter('.'));
        assert!(!DummyMatcher::is_latin_letter(' '));
        assert!(!DummyMatcher::is_latin_letter('\u{6211}'));
        assert!(!DummyMatcher::is_latin_letter('\u{306E}'));
    }

    #[test]
    fn test_matches_with_surrounding_latin_chars() {
        let contexts = vec![
            NumberContext::new("abc", "def"),
            NumberContext::new("abc", ""),
            NumberContext::new("", "def"),
            NumberContext::new("\u{00C9}", ""),
            NumberContext::new("e\u{0301}", ""),
        ];
        find_matches_in_contexts(&contexts, false, true);
    }

    #[test]
    fn test_money_not_seen_as_phone_number() {
        let contexts = vec![
            NumberContext::new("$", ""),
            NumberContext::new("", "$"),
            NumberContext::new("\u{00A3}", ""),
            NumberContext::new("\u{00A5}", ""),
        ];
        find_matches_in_contexts(&contexts, false, true);
    }

    #[test]
    fn test_percentage_not_seen_as_phone_number() {
        let contexts = vec![NumberContext::new("", "%")];
        find_matches_in_contexts(&contexts, false, true);
    }

    #[test]
    fn test_phone_number_with_leading_or_trailing_money_matches() {
        let contexts = vec![
            NumberContext::new("$20 ", ""),
            NumberContext::new("", " 100$"),
        ];
        find_matches_in_contexts(&contexts, true, true);
    }

    #[test]
    fn test_matches_with_surrounding_latin_chars_and_leading_punctuation() {
        let possible_only_contexts = vec![
            NumberContext::new("abc", "def"),
            NumberContext::new("", "def"),
            NumberContext::new("", "\u{00C9}"),
        ];

        let number_with_plus = "+14156667777";
        let number_with_brackets = "(415)6667777";
        find_matches_in_contexts_with_region_number(
            &possible_only_contexts,
            false,
            true,
            Some(Region::US),
            number_with_plus,
        );
        find_matches_in_contexts_with_region_number(
            &possible_only_contexts,
            false,
            true,
            Some(Region::US),
            number_with_brackets,
        );

        let valid_contexts = vec![
            NumberContext::new("abc", ""),
            NumberContext::new("\u{00C9}", ""),
            NumberContext::new("\u{00C9}", "."),
            NumberContext::new("\u{00C9}", " def"),
        ];

        find_matches_in_contexts_with_region_number(
            &valid_contexts,
            true,
            true,
            Some(Region::US),
            number_with_plus,
        );
        find_matches_in_contexts_with_region_number(
            &valid_contexts,
            true,
            true,
            Some(Region::US),
            number_with_brackets,
        );
    }

    #[test]
    fn test_matches_with_surrounding_chinese_chars() {
        let valid_contexts = vec![
            NumberContext::new(
                "\u{6211}\u{7684}\u{7535}\u{8BDD}\u{53F7}\u{7801}\u{662F}",
                "",
            ),
            NumberContext::new(
                "",
                "\u{662F}\u{6211}\u{7684}\u{7535}\u{8BDD}\u{53F7}\u{7801}",
            ),
            NumberContext::new(
                "\u{8BF7}\u{62E8}\u{6253}",
                "\u{6211}\u{5728}\u{660E}\u{5929}",
            ),
        ];
        find_matches_in_contexts(&valid_contexts, true, true);
    }

    #[test]
    fn test_matches_with_surrounding_punctuation() {
        let valid_contexts = vec![
            NumberContext::new("My number-", ""),
            NumberContext::new("", ".Nice day."),
            NumberContext::new("Tel:", "."),
            NumberContext::new("Tel: ", " on Saturdays."),
        ];
        find_matches_in_contexts(&valid_contexts, true, true);
    }

    #[test]
    fn test_matches_multiple_phone_numbers_separated_by_phone_number_punctuation() {
        let text = "Call 650-253-4561 -- 455-234-3451";
        let region = Some(Region::US);

        let mut number1 = PhoneNumber::default();
        number1.country_code = phone_util()
            .get_country_code_for_region(Region::US)
            .unwrap();
        number1.national_number = 6502534561;

        let mut number2 = PhoneNumber::default();
        number2.country_code = phone_util()
            .get_country_code_for_region(Region::US)
            .unwrap();
        number2.national_number = 4552343451;

        let mut matches = find_numbers(text, region);

        let match1 = matches.next().unwrap();
        assert_eq!(5, match1.start);
        assert_eq!("650-253-4561", match1.raw_string);
        assert_eq!(&number1, &match1.number);

        let match2 = matches.next().unwrap();
        assert_eq!(21, match2.start);
        assert_eq!("455-234-3451", match2.raw_string);
        assert_eq!(&number2, &match2.number);
    }

    #[test]
    fn test_does_not_match_multiple_phone_numbers_separated_with_no_white_space() {
        let text = "Call 650-253-4561--455-234-3451";
        assert!(has_no_matches(find_numbers(text, Some(Region::US))));
    }

    #[test]
    fn test_non_matching_brackets_are_invalid() {
        assert!(has_no_matches(find_numbers(
            "80.585[79.964, 81.191]",
            Some(Region::US)
        )));
        assert!(has_no_matches(find_numbers(
            "80.585 [79.964]",
            Some(Region::US)
        )));
        assert!(has_no_matches(find_numbers(
            "80.585 ((79.964)",
            Some(Region::US)
        )));
        assert!(has_no_matches(find_numbers(
            "(80).(585) (79).(9)64",
            Some(Region::US)
        )));
    }

    #[test]
    fn test_no_match_if_region_is_null() {
        assert!(has_no_matches(find_numbers(
            "Random text body - number is 0331 6005, see you there",
            None
        )));
    }

    #[test]
    fn test_no_match_in_empty_string() {
        assert!(has_no_matches(find_numbers("", Some(Region::US))));
        assert!(has_no_matches(find_numbers("  ", Some(Region::US))));
    }

    #[test]
    fn test_no_match_if_no_number() {
        assert!(has_no_matches(find_numbers(
            "Random text body - number is foobar, see you there",
            Some(Region::US)
        )));
    }

    #[test]
    fn test_sequences() {
        let text = "Call 033316005  or 032316005!";
        let region = Region::NZ;

        let mut number1 = PhoneNumber::default();
        number1.country_code = phone_util().get_country_code_for_region(region).unwrap();
        number1.national_number = 33316005;

        let mut number2 = PhoneNumber::default();
        number2.country_code = phone_util().get_country_code_for_region(region).unwrap();
        number2.national_number = 32316005;

        let mut matches = find_numbers_for_leniency(text, Some(region), Leniency::Possible);

        let match1 = matches.next().unwrap();
        assert_eq!(5, match1.start);
        assert_eq!("033316005", match1.raw_string);
        assert_eq!(&number1, &match1.number);

        let match2 = matches.next().unwrap();
        assert_eq!(19, match2.start);
        assert_eq!("032316005", match2.raw_string);
        assert_eq!(&number2, &match2.number);
    }

    #[test]
    fn test_null_input() {
        assert!(has_no_matches(find_numbers("foobar", None)));
    }

    #[test]
    fn test_max_matches() {
        let mut numbers = String::new();
        for _ in 0..100 {
            numbers.push_str("My info: 415-666-7777,");
        }

        let number = phone_util().parse("+14156667777", None).unwrap();
        let expected: Vec<PhoneNumber> = vec![number; 100];

        let iterable = get_phone_matcher_factory().create_matcher(
            &numbers,
            Leniency::Valid,
            10,
            Some(Region::US),
        );
        let mut actual = Vec::with_capacity(100);
        for match_ in iterable {
            actual.push(match_.number.clone());
        }
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_max_matches_invalid() {
        let mut numbers = String::new();
        for _ in 0..10 {
            numbers.push_str("My address 949-8945-0");
        }
        for _ in 0..100 {
            numbers.push_str("My info: 415-666-7777,");
        }

        let mut iterable = get_phone_matcher_factory().create_matcher(
            &numbers,
            Leniency::Valid,
            10,
            Some(Region::US),
        );
        assert!(iterable.next().is_none());
    }

    #[test]
    fn test_max_matches_mixed() {
        let mut numbers = String::new();
        for _ in 0..100 {
            numbers.push_str("My info: 415-666-7777 123 fake street");
        }

        let number = phone_util().parse("+14156667777", None).unwrap();
        let expected: Vec<PhoneNumber> = vec![number; 10]; // due to max matches

        let iterable = get_phone_matcher_factory().create_matcher(
            &numbers,
            Leniency::Valid,
            10,
            Some(Region::US),
        );
        let mut actual = Vec::new();
        for match_ in iterable {
            actual.push(match_.number.clone());
        }
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_non_plus_prefixed_numbers_not_found_for_invalid_region() {
        let mut iterable = find_numbers("1 456 764 156", None); // Using None for RegionCode.ZZ
        assert!(iterable.next().is_none());
    }

    #[test]
    fn test_empty_iteration() {
        let mut iterable = find_numbers("", None);
        assert!(iterable.next().is_none());
        assert!(iterable.next().is_none());
    }

    #[test]
    fn test_single_iteration() {
        let mut iterable = find_numbers("+14156667777", None);
        assert!(iterable.next().is_some());
        assert!(iterable.next().is_none());
    }

    #[test]
    fn test_double_iteration() {
        let mut iterable = find_numbers("+14156667777 foobar +14156667777 ", None);
        assert!(iterable.next().is_some());
        assert!(iterable.next().is_some());
        assert!(iterable.next().is_none());
    }

    #[test]
    fn test_removal_not_supported() {
        // Iterator.remove() отсутствует в стандартном интерфейсе Iterator языка Rust.
    }

    // ──────────────────────────── Static test cases ────────────────────────────

    struct NumberTest {
        raw_string: &'static str,
        region: Region,
    }

    impl NumberTest {
        fn new(raw_string: &'static str, region: Region) -> Self {
            Self { raw_string, region }
        }
    }

    fn get_impossible_cases() -> Vec<NumberTest> {
        vec![
            NumberTest::new("12345", Region::US),
            NumberTest::new("23456789", Region::US),
            NumberTest::new("234567890112", Region::US),
            NumberTest::new("650+253+1234", Region::US),
            NumberTest::new("3/10/1984", Region::CA),
            NumberTest::new("03/27/2011", Region::US),
            NumberTest::new("31/8/2011", Region::US),
            NumberTest::new("1/12/2011", Region::US),
            NumberTest::new("10/12/82", Region::DE),
            NumberTest::new("650x2531234", Region::US),
            NumberTest::new("2012-01-02 08:00", Region::US),
            NumberTest::new("2012/01/02 08:00", Region::US),
            NumberTest::new("20120102 08:00", Region::US),
            NumberTest::new("2014-04-12 04:04 PM", Region::US),
            NumberTest::new("2014-04-12 &nbsp;04:04 PM", Region::US),
            NumberTest::new("2014-04-12 &nbsp;04:04 PM", Region::US),
            NumberTest::new("2014-04-12  04:04 PM", Region::US),
        ]
    }

    fn get_possible_only_cases() -> Vec<NumberTest> {
        vec![
            NumberTest::new("7121115678", Region::US),
            NumberTest::new("1650 x 253 - 1234", Region::US),
            NumberTest::new("650 x 253 - 1234", Region::US),
            NumberTest::new("6502531x234", Region::US),
            NumberTest::new("(20) 3346 1234", Region::GB),
        ]
    }

    fn get_valid_cases() -> Vec<NumberTest> {
        vec![
            NumberTest::new("65 02 53 00 00", Region::US),
            NumberTest::new("6502 538365", Region::US),
            NumberTest::new("650//253-1234", Region::US),
            NumberTest::new("650/253/1234", Region::US),
            NumberTest::new("9002309. 158", Region::US),
            NumberTest::new("12 7/8 - 14 12/34 - 5", Region::US),
            NumberTest::new("12.1 - 23.71 - 23.45", Region::US),
            NumberTest::new("800 234 1 111x1111", Region::US),
            NumberTest::new("1979-2011 100", Region::US),
            NumberTest::new("+494949-4-94", Region::DE),
            NumberTest::new(
                "\u{FF14}\u{FF11}\u{FF15}\u{FF16}\u{FF16}\u{FF16}\u{FF16}-\u{FF17}\u{FF17}\u{FF17}",
                Region::US,
            ),
            NumberTest::new("2012-0102 08", Region::US),
            NumberTest::new("2012-01-02 08", Region::US),
            NumberTest::new("1800-1-0-10 22", Region::AU),
            NumberTest::new("030-3-2 23 12 34", Region::DE),
            NumberTest::new("03 0 -3 2 23 12 34", Region::DE),
            NumberTest::new("(0)3 0 -3 2 23 12 34", Region::DE),
            NumberTest::new("0 3 0 -3 2 23 12 34", Region::DE),
            NumberTest::new("+52 332 123 23 23", Region::MX),
        ]
    }

    fn get_strict_grouping_cases() -> Vec<NumberTest> {
        vec![
            NumberTest::new("(415) 6667777", Region::US),
            NumberTest::new("415-6667777", Region::US),
            NumberTest::new("0800-2491234", Region::DE),
            NumberTest::new("0900-1 123123", Region::DE),
            NumberTest::new("(0)900-1 123123", Region::DE),
            NumberTest::new("0 900-1 123123", Region::DE),
            NumberTest::new("+33 3 34 2312", Region::FR),
        ]
    }

    fn get_exact_grouping_cases() -> Vec<NumberTest> {
        vec![
            NumberTest::new(
                "\u{FF14}\u{FF11}\u{FF15}\u{FF16}\u{FF16}\u{FF16}\u{FF17}\u{FF17}\u{FF17}\u{FF17}",
                Region::US,
            ),
            NumberTest::new(
                "\u{FF14}\u{FF11}\u{FF15}-\u{FF16}\u{FF16}\u{FF16}-\u{FF17}\u{FF17}\u{FF17}\u{FF17}",
                Region::US,
            ),
            NumberTest::new("4156667777", Region::US),
            NumberTest::new("4156667777 x 123", Region::US),
            NumberTest::new("415-666-7777", Region::US),
            NumberTest::new("415/666-7777", Region::US),
            NumberTest::new("415-666-7777 ext. 503", Region::US),
            NumberTest::new("1 415 666 7777 x 123", Region::US),
            NumberTest::new("+1 415-666-7777", Region::US),
            NumberTest::new("+494949 49", Region::DE),
            NumberTest::new("+49-49-34", Region::DE),
            NumberTest::new("+49-4931-49", Region::DE),
            NumberTest::new("04931-49", Region::DE),
            NumberTest::new("+49-494949", Region::DE),
            NumberTest::new("+49-494949 ext. 49", Region::DE),
            NumberTest::new("+49494949 ext. 49", Region::DE),
            NumberTest::new("0494949", Region::DE),
            NumberTest::new("0494949 ext. 49", Region::DE),
            NumberTest::new("01 (33) 3461 2234", Region::MX),
            NumberTest::new("(33) 3461 2234", Region::MX),
            NumberTest::new("1800-10-10 22", Region::AU),
            NumberTest::new("0900-1 123 123", Region::DE),
            NumberTest::new("(0)900-1 123 123", Region::DE),
            NumberTest::new("0 900-1 123 123", Region::DE),
            NumberTest::new("+33 3 34 23 12", Region::FR),
        ]
    }

    #[test]
    fn test_matches_with_possible_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_strict_grouping_cases());
        test_cases.extend(get_exact_grouping_cases());
        test_cases.extend(get_valid_cases());
        test_cases.extend(get_possible_only_cases());
        do_test_number_matches_for_leniency(&test_cases, Leniency::Possible);
    }

    #[test]
    fn test_non_matches_with_possible_leniency() {
        let test_cases = get_impossible_cases();
        do_test_number_non_matches_for_leniency(&test_cases, Leniency::Possible);
    }

    #[test]
    fn test_matches_with_valid_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_strict_grouping_cases());
        test_cases.extend(get_exact_grouping_cases());
        test_cases.extend(get_valid_cases());
        do_test_number_matches_for_leniency(&test_cases, Leniency::Valid);
    }

    #[test]
    fn test_non_matches_with_valid_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_impossible_cases());
        test_cases.extend(get_possible_only_cases());
        do_test_number_non_matches_for_leniency(&test_cases, Leniency::Valid);
    }

    #[test]
    fn test_matches_with_strict_grouping_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_strict_grouping_cases());
        test_cases.extend(get_exact_grouping_cases());
        do_test_number_matches_for_leniency(&test_cases, Leniency::StrictGrouping);
    }

    #[test]
    fn test_non_matches_with_strict_group_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_impossible_cases());
        test_cases.extend(get_possible_only_cases());
        test_cases.extend(get_valid_cases());
        do_test_number_non_matches_for_leniency(&test_cases, Leniency::StrictGrouping);
    }

    #[test]
    fn test_matches_with_exact_grouping_leniency() {
        let test_cases = get_exact_grouping_cases();
        do_test_number_matches_for_leniency(&test_cases, Leniency::ExactGrouping);
    }

    #[test]
    fn test_non_matches_exact_group_leniency() {
        let mut test_cases = vec![];
        test_cases.extend(get_impossible_cases());
        test_cases.extend(get_possible_only_cases());
        test_cases.extend(get_valid_cases());
        test_cases.extend(get_strict_grouping_cases());
        do_test_number_non_matches_for_leniency(&test_cases, Leniency::ExactGrouping);
    }

    fn do_test_number_matches_for_leniency(test_cases: &[NumberTest], leniency: Leniency) {
        let mut no_match_found_count = 0;
        let mut wrong_match_found_count = 0;
        for test in test_cases {
            let mut iterator =
                find_numbers_for_leniency(test.raw_string, Some(test.region), leniency);
            if let Some(match_) = iterator.next() {
                if test.raw_string != match_.raw_string {
                    wrong_match_found_count += 1;
                    eprintln!(
                        "Found wrong match in test {:?}. Found {}",
                        test.raw_string, match_.raw_string
                    );
                }
            } else {
                no_match_found_count += 1;
                eprintln!(
                    "No match found in {:?} for leniency: {:?}",
                    test.raw_string, leniency
                );
            }
        }
        assert_eq!(0, no_match_found_count);
        assert_eq!(0, wrong_match_found_count);
    }

    fn do_test_number_non_matches_for_leniency(test_cases: &[NumberTest], leniency: Leniency) {
        let mut match_found_count = 0;
        for test in test_cases {
            let mut iterator =
                find_numbers_for_leniency(test.raw_string, Some(test.region), leniency);
            if iterator.next().is_some() {
                match_found_count += 1;
                eprintln!(
                    "Match found in {:?} for leniency: {:?}",
                    test.raw_string, leniency
                );
            }
        }
        assert_eq!(0, match_found_count);
    }

    // ──────────────────────────── Helper functions ────────────────────────────

    struct NumberContext {
        leading_text: &'static str,
        trailing_text: &'static str,
    }

    impl NumberContext {
        fn new(leading_text: &'static str, trailing_text: &'static str) -> Self {
            Self {
                leading_text,
                trailing_text,
            }
        }
    }

    fn do_test_find_in_context(number: &str, default_country: Option<Region>) {
        find_possible_in_context(number, default_country);

        if let Ok(parsed) = phone_util().parse(number, default_country)
            && phone_util().is_valid_number(&parsed).unwrap()
        {
            find_valid_in_context(number, default_country);
        }
    }

    fn find_possible_in_context(number: &str, default_country: Option<Region>) {
        let contexts = vec![
            NumberContext::new("", ""),
            NumberContext::new("   ", "\t"),
            NumberContext::new("Hello ", ""),
            NumberContext::new("", " to call me!"),
            NumberContext::new("Hi there, call ", " to reach me!"),
            NumberContext::new("Hi there, call ", ", or don't"),
            NumberContext::new("Hi call", ""),
            NumberContext::new("", "forme"),
            NumberContext::new("Hi call", "forme"),
            NumberContext::new("It's cheap! Call ", " before 6:30"),
            NumberContext::new("Call ", " or +1800-123-4567!"),
            NumberContext::new("Call me on June 2 at", ""),
            NumberContext::new("As quoted by Alfonso 12-15 (2009), you may call me at ", ""),
            NumberContext::new(
                "As quoted by Alfonso et al. 12-15 (2009), you may call me at ",
                "",
            ),
            NumberContext::new("As I said on 03/10/2011, you may call me at ", ""),
            NumberContext::new("", ", 45 days a year"),
            NumberContext::new("", ";x 7246433"),
            NumberContext::new("Call ", "/x12 more"),
        ];
        do_test_in_context(number, default_country, &contexts, Leniency::Possible);
    }

    fn find_valid_in_context(number: &str, default_country: Option<Region>) {
        let contexts = vec![
            NumberContext::new("It's only 9.99! Call ", " to buy"),
            NumberContext::new("Call me on 21.6.1984 at ", ""),
            NumberContext::new("Call me on 06/21 at ", ""),
            NumberContext::new("Call me on 21.6. at ", ""),
            NumberContext::new("Call me on 06/21/84 at ", ""),
        ];
        do_test_in_context(number, default_country, &contexts, Leniency::Valid);
    }

    fn do_test_in_context(
        number: &str,
        default_country: Option<Region>,
        context_pairs: &[NumberContext],
        leniency: Leniency,
    ) {
        for context in context_pairs {
            let text = format!(
                "{}{}{}",
                context.leading_text, number, context.trailing_text
            );

            let start = context.leading_text.len();
            let end = start + number.len();
            let mut iterator = find_numbers_for_leniency(&text, default_country, leniency);

            let match_ = iterator.next();
            assert!(
                match_.is_some(),
                "Did not find a number in '{}'; expected '{}'",
                text,
                number
            );
            let match_ = match_.unwrap();

            let extracted = &text[match_.start..match_.end()];
            assert!(
                start == match_.start && end == match_.end(),
                "Unexpected phone region in '{}'; extracted '{}', expected: {}",
                text,
                extracted,
                number
            );
            assert_eq!(number, extracted);
            assert_eq!(match_.raw_string, extracted);
        }
    }

    fn assert_equal_range(text: &str, index: usize, start: usize, end: usize) {
        let sub = &text[index..];
        let mut matches = find_numbers_for_leniency(sub, Some(Region::NZ), Leniency::Possible);
        let match_ = matches.next().unwrap();
        assert_eq!(start - index, match_.start);
        assert_eq!(end - index, match_.end());
        assert_eq!(&sub[match_.start..match_.end()], match_.raw_string);
    }

    fn assert_match_properties(
        match_: Option<PhoneNumberMatch>,
        text: &str,
        number: &str,
        region: Option<Region>,
    ) {
        let expected_result = phone_util().parse(number, region).unwrap();
        assert!(
            match_.is_some(),
            "Did not find a number in '{}'; expected {}",
            text,
            number
        );
        let match_ = match_.unwrap();
        assert_eq!(&expected_result, &match_.number);
        assert_eq!(number, match_.raw_string);
    }

    fn find_matches_in_contexts(contexts: &[NumberContext], is_valid: bool, is_possible: bool) {
        find_matches_in_contexts_with_region_number(
            contexts,
            is_valid,
            is_possible,
            Some(Region::US),
            "415-666-7777",
        );
    }

    fn find_matches_in_contexts_with_region_number(
        contexts: &[NumberContext],
        is_valid: bool,
        is_possible: bool,
        region: Option<Region>,
        number: &str,
    ) {
        if is_valid {
            do_test_in_context(number, region, contexts, Leniency::Valid);
        } else {
            for context in contexts {
                let text = format!(
                    "{}{}{}",
                    context.leading_text, number, context.trailing_text
                );
                assert!(
                    has_no_matches(find_numbers(&text, region)),
                    "Should not have found a number in {}",
                    text
                );
            }
        }
        if is_possible {
            do_test_in_context(number, region, contexts, Leniency::Possible);
        } else {
            for context in contexts {
                let text = format!(
                    "{}{}{}",
                    context.leading_text, number, context.trailing_text
                );
                assert!(
                    has_no_matches(find_numbers_for_leniency(&text, region, Leniency::Possible)),
                    "Should not have found a number in {}",
                    text
                );
            }
        }
    }

    // =========================================================================
    // Subset region detection  (.regions([…]))
    // =========================================================================

    fn find_numbers_subset<'a>(
        text: &'a str,
        regions: &[Region],
        hint: Option<Region>,
    ) -> PhoneNumberMatcher<'a, PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal> {
        get_phone_matcher_factory().create_matcher_with_regions(
            text,
            Leniency::Valid,
            u64::MAX,
            regions.iter().copied(),
            hint,
        )
    }

    /// International (`+`) numbers must be found even when the subset is small,
    /// because they self-identify via their country code.
    #[test]
    fn test_subset_region_finds_intl() {
        let text = "Call +44 20 7946 0958 now.";
        let m = find_numbers_subset(text, &[Region::US], None).next();
        assert!(
            m.is_some(),
            "subset should still match international numbers even if country not in subset"
        );
        assert_eq!(44, m.unwrap().number.country_code);
    }

    /// A national-format number inside the subset is matched.
    #[test]
    fn test_subset_region_matches_number_in_subset() {
        let text = "Call 020 7946 0958.";
        let m = find_numbers_subset(text, &[Region::GB], None).next();
        assert!(m.is_some(), "number in subset should be matched");
        assert_eq!(44, m.unwrap().number.country_code);
    }

    /// The same digits with a different subset are either attributed to a
    /// different country code or not matched at all — the subset fully controls
    /// national resolution.
    #[test]
    fn test_subset_region_excludes_out_of_subset() {
        let text = "020 7946 0958";
        // US subset: these digits don't form a valid US number.
        let no_match = find_numbers_subset(text, &[Region::US], None).next();
        assert!(
            no_match.is_none(),
            "number that is not valid in any subset region must not be returned"
        );
    }

    /// Order of regions passed to `.regions()` doesn't matter (they are sorted
    /// internally), but the MRU hint still takes priority.
    #[test]
    fn test_subset_region_mru_hint_respected() {
        let text = "020 7183 8750";
        // AR is before GB in sorted order, but GB is the hint → should win.
        let m_gb = find_numbers_subset(text, &[Region::AR, Region::GB], Some(Region::GB)).next();
        let m_ar = find_numbers_subset(text, &[Region::AR, Region::GB], Some(Region::AR)).next();
        assert_eq!(44, m_gb.unwrap().number.country_code);
        assert_eq!(54, m_ar.unwrap().number.country_code);
    }

    /// MRU state is updated across successive matches: first a number seeds the
    /// MRU, then the second one follows it.
    #[test]
    fn test_subset_region_mru_propagates_across_matches() {
        let text = "First: +44 20 7946 0958. Then: 020 7183 8750.";
        // AR is in the subset, but GB should win because the first (+44) number
        // seeds the MRU to GB.
        let codes: Vec<i32> =
            find_numbers_subset(text, &[Region::AR, Region::GB], None)
                .map(|m| m.number.country_code)
                .collect();
        assert_eq!(vec![44, 44], codes);
    }

    /// Builder entry-point: `.regions([…])` and `.regions_with_hint(…, hint)`
    /// produce the same results as the factory method.
    #[test]
    fn test_subset_region_builder_methods() {
        let text = "020 7183 8750";

        let via_regions: Vec<i32> = get_phone_matcher_factory()
            .matcher_builder(text)
            .regions([Region::GB])
            .build()
            .map(|m| m.number.country_code)
            .collect();
        assert_eq!(vec![44], via_regions);

        // hint for a different region in subset → AR wins
        let via_hint: Vec<i32> = get_phone_matcher_factory()
            .matcher_builder(text)
            .regions_with_hint([Region::AR, Region::GB], Region::AR)
            .build()
            .map(|m| m.number.country_code)
            .collect();
        assert_eq!(vec![54], via_hint);
    }

    /// Passing duplicate regions to `.regions()` must not cause duplicate
    /// attempts or wrong results.
    #[test]
    fn test_subset_region_dedup() {
        let text = "Call 020 7946 0958.";
        let codes: Vec<i32> = get_phone_matcher_factory()
            .matcher_builder(text)
            // GB appears twice — should still work correctly and find it once.
            .regions([Region::GB, Region::GB, Region::US])
            .build()
            .map(|m| m.number.country_code)
            .collect();
        assert_eq!(vec![44], codes);
    }

    // =========================================================================
    // Automatic region detection
    // =========================================================================

    /// Builds an auto-region matcher (no fixed region), optionally seeded with
    /// an initial most-recently-used region.
    fn find_numbers_auto(
        text: &str,
        initial_region: Option<Region>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtilInternal, &'static PhoneNumberUtilInternal> {
        get_phone_matcher_factory().create_matcher_auto(
            text,
            Leniency::Valid,
            u64::MAX,
            initial_region,
        )
    }

    fn country_codes_auto(text: &str, initial_region: Option<Region>) -> Vec<i32> {
        find_numbers_auto(text, initial_region)
            .map(|m| m.number.country_code)
            .collect()
    }

    /// International (`+`) numbers carry their own country code, so auto
    /// detection must resolve them identically to the classic "no region"
    /// matcher — regardless of how many regions exist.
    #[test]
    fn test_auto_region_matches_international_like_none() {
        let text = "Call +1 (650) 333-6000 or +44 20 7946 0958.";

        let auto = country_codes_auto(text, None);
        let fixed_none: Vec<i32> = find_numbers(text, None)
            .map(|m| m.number.country_code)
            .collect();

        assert_eq!(auto, fixed_none);
        assert_eq!(auto, vec![1, 44]);
    }

    /// A national-format number is found without any region being supplied, and
    /// the resulting number is valid (even if its region is ambiguous on its
    /// own).
    #[test]
    fn test_auto_region_finds_national_without_seed() {
        let text = "Reach me at 020 7946 0958.";
        let match_ = find_numbers_auto(text, None).next();

        assert!(
            match_.is_some(),
            "auto region should find a national number even without a seed"
        );
        let match_ = match_.unwrap();
        assert_eq!("020 7946 0958", match_.raw_string);
        assert!(
            phone_util().is_valid_number(&match_.number).unwrap(),
            "the auto-detected number must be valid in the region that matched"
        );
    }

    /// When a region is seeded, a national-format number that is valid there is
    /// attributed to that region (it is tried first).
    #[test]
    fn test_auto_region_national_uses_seed() {
        let text = "Call 020 7946 0958 today";
        let match_ = find_numbers_auto(text, Some(Region::GB)).next();
        assert_match_properties(match_, text, "020 7946 0958", Some(Region::GB));
    }

    /// The most-recently-used region carries across candidates: an
    /// international number fixes the locale, and a following ambiguous
    /// national number is then attributed to the same region rather than to an
    /// arbitrary colliding one.
    #[test]
    fn test_auto_region_mru_locality() {
        let text = "Main +44 20 7946 0958, alt 020 7183 8750.";
        assert_eq!(country_codes_auto(text, None), vec![44, 44]);
    }

    /// The same ambiguous digit string is attributed to different regions
    /// depending on the seed, demonstrating that collisions are resolved by
    /// context (the MRU/seed) and that the number itself is never rewritten to
    /// force a region.
    #[test]
    fn test_auto_region_collision_resolved_by_context() {
        let text = "Ambiguous 020 7183 8750 here.";

        let as_gb = find_numbers_auto(text, Some(Region::GB)).next().unwrap();
        let as_ar = find_numbers_auto(text, Some(Region::AR)).next().unwrap();

        // The very same digits are valid in both regions; the seed decides.
        assert_eq!(44, as_gb.number.country_code);
        assert_eq!(54, as_ar.number.country_code);
        // Same source text, different attribution purely from context.
        assert_eq!(as_gb.raw_string, as_ar.raw_string);
    }

    /// Auto detection does not invent matches in plain prose.
    #[test]
    fn test_auto_region_no_false_positive_on_text() {
        let text = "Just some ordinary words, without any phone numbers at all.";
        assert!(has_no_matches(find_numbers_auto(text, None)));
    }

    /// The builder entry points (`auto_region` / `auto_region_with_hint`) wire
    /// up to the same behaviour as the direct factory method.
    #[test]
    fn test_auto_region_builder() {
        let text = "Main +44 20 7946 0958, alt 020 7183 8750.";
        let codes: Vec<i32> = get_phone_matcher_factory()
            .matcher_builder(text)
            .auto_region()
            .build()
            .map(|m| m.number.country_code)
            .collect();
        assert_eq!(codes, vec![44, 44]);

        let seeded: Vec<i32> = get_phone_matcher_factory()
            .matcher_builder("020 7183 8750")
            .auto_region_with_hint(Region::AR)
            .build()
            .map(|m| m.number.country_code)
            .collect();
        assert_eq!(seeded, vec![54]);
    }
}
