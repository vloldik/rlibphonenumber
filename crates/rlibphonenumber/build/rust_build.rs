use prost_build::Config;
use uniprops_gen::UnipropsBuilder;

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
        .out_file("uniprops_without_nl.rs")
        .with_digits(false)
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
