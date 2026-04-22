use prost_build::Config;
use uniprops_gen::{LookupStrategy, UnipropsBuilder};

fn main() {
    let deprecated = concat!(
        "#[deprecated(note = \"This field is shadowed by the wrapper and is intentionally ",
        "left empty. Access the underlying data via `.original`.\")]"
    );

    let mut config = Config::new();
    for field in &[
        "general_desc",
        "fixed_line",
        "mobile",
        "toll_free",
        "premium_rate",
        "shared_cost",
        "personal_number",
        "voip",
        "pager",
        "uan",
        "emergency",
        "voicemail",
        "short_code",
        "standard_rate",
        "carrier_specific",
        "sms_services",
        "no_international_dialling",
        "number_format",
        "intl_number_format",
        "leading_digits",
        "international_prefix",
        "national_prefix_for_parsing",
    ] {
        config.field_attribute(
            format!("i18n.phonenumbers.PhoneMetadata.{field}"),
            deprecated,
        );
    }

    config.field_attribute(
        "i18n.phonenumbers.PhoneNumberDesc.national_number_pattern",
        deprecated,
    );
    for field in &["leading_digits_pattern", "pattern"] {
        config.field_attribute(
            format!("i18n.phonenumbers.NumberFormat.{field}"),
            deprecated,
        );
    }

    config
        .compile_protos(
            &[
                "resources/phonemetadata.proto",
                "resources/phonenumber.proto",
            ],
            &["resources/"],
        )
        .unwrap();

    UnipropsBuilder::new()
        .with_categories(false)
        .out_file("uniprops_digits.rs")
        .build();

    // [^p{N}p{L}]
    // For others, since char is valid unicode Category::from_char should be Some()
    UnipropsBuilder::new()
        .filter(|r| !r.general_category.starts_with(['N', 'L']))
        .with_lookup_strategy(if cfg!(feature = "lite") {
            LookupStrategy::BSearch
        } else {
            LookupStrategy::Trie { shift: 8 }
        })
        .out_file("uniprops_without_nl.rs")
        .with_digits(false)
        .build();

    // Latin letters
    UnipropsBuilder::new()
        .with_digits(false)
        .with_lookup_strategy(LookupStrategy::BSearch)
        .filter(|record| {
            let cp = record.code_point;
            let is_in_latin_block = (cp <= 0x007F) || // Basic Latin
                (0x0080..=0x00FF).contains(&cp) || // Latin-1 Supplement
                (0x0100..=0x017F).contains(&cp) || // Latin Extended-A
                (0x0180..=0x024F).contains(&cp) || // Latin Extended-B
                (0x1E00..=0x1EFF).contains(&cp) || // Latin Extended Additional
                (0x0300..=0x036F).contains(&cp); // Combining Diacritical Marks

            if !is_in_latin_block {
                return false;
            }

            let cat = &record.general_category;
            let is_alpha = cat.starts_with('L');
            let is_non_spacing_mark = cat == "Mn";

            is_alpha || is_non_spacing_mark
        })
        .out_file("uniprops_latin_letters.rs")
        .build();
    UnipropsBuilder::new()
        .with_digits(false)
        .with_lookup_strategy(LookupStrategy::BSearch)
        .filter(|record| record.general_category == "Sc")
        .out_file("uniprops_currencies.rs")
        .build();

    if cfg!(all(feature = "lite", not(feature = "regex"))) {
        UnipropsBuilder::new()
            .with_digits(false)
            .with_categories(false)
            .filter(|r| r.general_category == "Nd" && r.decimal_digit_value == Some(0))
            .with_custom(|recs| {
                let decimals: String = recs
                    .iter()
                    .map(|r| format!(r"\u{{{:x}}}-\u{{{:x}}}", r.code_point, r.code_point + 9))
                    .collect();
                format!("pub const DIGITS_ND: &str = \"{decimals}\";")
            })
            .out_file("uniprops_digits_pat.rs")
            .build();
        UnipropsBuilder::new()
            .with_digits(false)
            .with_categories(false)
            .filter(|r| r.general_category.starts_with('Z'))
            .with_custom(|recs| {
                let decimals: String = recs
                    .iter()
                    .map(|r| format!(r"\u{{{:x}}}", r.code_point))
                    .collect();
                format!("pub const SEPARATORS: &str = \"{decimals}\";")
            })
            .out_file("uniprops_separators_pat.rs")
            .build();
    }
}
