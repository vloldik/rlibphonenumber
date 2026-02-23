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

fn main() {
    struct GenWarnings {}

    impl CustomizeCallback for GenWarnings {
        fn file(&self, _file: &protobuf::reflect::FileDescriptor) -> Customize {
            Customize::default().before("#![allow(deprecated)]")
        }
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
                ]
                .iter()
                .any(|name| *name == field_proto.name())
            {
                Customize::default().before(
                    "#[deprecated(note = \"This field is shadowed by the wrapper and is intentionally left empty. Access the underlying data via `.original`.\")]",
                )
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
}
