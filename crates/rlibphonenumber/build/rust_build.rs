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

use protobuf::reflect::FieldDescriptor;
use protobuf_codegen::{Customize, CustomizeCallback};
use uniprops_gen::UnipropsBuilder;

fn main() {
    struct GenWarnings {}

    impl CustomizeCallback for GenWarnings {
        fn field(&self, field: &FieldDescriptor) -> Customize {
            let field_proto = field.proto();
            if field.containing_message().name() == "PhoneMetadata"
                && [
                    //PhoneNumberDescWrapper
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
                    //Vec<NumberFormatWrapper>
                    "number_format",
                    "intl_number_format",
                    // Regexp
                    "leading_digits",
                    "international_prefix",
                    "national_prefix_for_parsing",
                ]
                .contains(&field_proto.name())
                || field.containing_message().name() == "PhoneNumberDesc"
                    && field_proto.name() == "national_number_pattern"
                || field.containing_message().name() == "NumberFormat"
                    && ["leading_digits_pattern", "pattern"].contains(&field_proto.name())
            {
                Customize::default().before(
                    "#[deprecated(note = \"This field is shadowed by the wrapper and is intentionally left empty. Access the underlying data via `.original`.\")]",
                ).generate_getter(false)
            } else {
                Default::default()
            }
        }
    }

    protobuf_codegen::Codegen::new()
        .pure()
        .includes(["resources"])
        .input("resources/phonemetadata.proto")
        .input("resources/phonenumber.proto")
        .cargo_out_dir("proto_gen")
        .customize_callback(GenWarnings {})
        .run_from_script();

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
                format!("pub const DIGITS_ND: &str = \"[{}]\";", decimals)
            })
            .out_file("uniprops_digits_pat.rs")
            .build();
    }
}
