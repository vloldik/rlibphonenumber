use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use xml::{EventReader, reader::XmlEvent};

struct CountryData {
    name: String,
    code: String,
}

fn get_country_list_from_metadata() -> HashSet<String> {
    let file = include_str!(concat!(env!("OUT_DIR"), "/PhoneNumberMetadata.xml"));
    let mut countries = HashSet::new();

    let parser = EventReader::from_str(file);
    for e in parser {
        if let Ok(XmlEvent::StartElement {
            name, attributes, ..
        }) = e
            && name.borrow().local_name == "territory"
            && let Some(id_attr) = attributes.iter().find(|attr| attr.name.local_name == "id")
        {
            let value = match id_attr.value.as_str() {
                "001" => continue,
                other => other,
            };
            countries.insert(value.to_string());
        }
    }

    countries
}

pub fn countries_enum(name: TokenStream) -> TokenStream {
    let metadata_countries = get_country_list_from_metadata();
    let csv_file = include_str!(concat!(env!("OUT_DIR"), "/countries.csv"));
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_file.as_bytes());

    let mut countries_data = Vec::new();
    let mut known_codes = HashSet::new();
    for record in rdr.records().flatten() {
        if record.len() >= 2 {
            let name = record[0].trim().to_string();
            let code = record[1].trim().to_uppercase();
            if code.len() == 2 {
                known_codes.insert(code.clone());
                countries_data.push(CountryData { name, code });
            }
        }
    }

    let mut missing_codes: Vec<_> = metadata_countries
        .iter()
        .filter(|code| !known_codes.contains(*code))
        .collect();

    missing_codes.sort();

    for code in missing_codes {
        countries_data.push(CountryData {
            name: format!(
                "Exceptionally reserved / Libphonenumber specific region ({})",
                code
            ),
            code: code.clone(),
        });
    }
    countries_data.sort_by(|a, b| a.code.cmp(&b.code));

    let variants = countries_data.iter().map(|country| {
        let ident = format_ident!("{}", country.code);
        let name_str = &country.name;

        let is_presented = if metadata_countries.contains(&country.code) {
            "YES"
        } else {
            "NO"
        };
        let doc_comment = format!("{}\n\npresented - **{}**", name_str, is_presented);

        quote! {
            #[doc = #doc_comment]
            #ident
        }
    });

    let str_mapper = countries_data.iter().map(|country| {
        let ident = format_ident!("{}", country.code);
        let code_str = &country.code;
        quote! { Self::#ident => #code_str }
    });

    let string_to_code_val = |s: &str| -> u16 {
        let bytes = s.as_bytes();
        let first = bytes[0].to_ascii_uppercase() as u16;
        let second = bytes[1].to_ascii_uppercase() as u16;
        (first << 8) | second
    };

    let match_cases = countries_data.iter().map(|country| {
        let ident = format_ident!("{}", country.code);
        let code_val = string_to_code_val(&country.code);
        quote! { #code_val => ::core::option::Option::Some(Self::#ident) }
    });

    let name = format_ident!("{}", name.to_string());

    quote! {
        #[derive(
            ::core::fmt::Debug,
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
            ::core::cmp::PartialOrd,
            ::core::cmp::Ord
        )]
        pub enum #name {
            #[doc = "Custom fallback variant for unrecognized country codes"]
            Custom([u8; 2]),
            #[doc = "001 - Non-Geographical Entity (World / Global Network)"]
            World,
            #(#variants),*
        }

        impl #name {
            #[inline]
            const fn string_to_code(bytes: &[u8]) -> ::core::option::Option<u16> {
                if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1].is_ascii_alphabetic() {
                    let first = bytes[0].to_ascii_uppercase() as u16;
                    let second = bytes[1].to_ascii_uppercase() as u16;
                    ::core::option::Option::Some((first << 8) | second)
                } else {
                    ::core::option::Option::None
                }
            }

            pub fn from_code(s: &str) -> ::core::option::Option<Self> {
                if s == "001" {
                    return ::core::option::Option::Some(Self::World);
                }
                let code = Self::string_to_code(s.as_bytes())?;
                match code {
                    #(#match_cases,)*
                    _ => {
                        let bytes = s.as_bytes();
                        if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1].is_ascii_alphabetic() {
                            ::core::option::Option::Some(Self::Custom([
                                bytes[0].to_ascii_uppercase(),
                                bytes[1].to_ascii_uppercase(),
                            ]))
                        } else {
                            ::core::option::Option::None
                        }
                    },
                }
            }
        }

        impl ::core::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    Self::Custom(bytes) => {
                        // SAFETY: custom string is guaranteed to contain valid ascii 
                        unsafe { ::core::str::from_utf8_unchecked(bytes) }
                    },
                    Self::World => "001",
                    #(#str_mapper),*
                }
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_ref())
            }
        }

        impl ::core::str::FromStr for #name {
            type Err = ::std::string::String;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                Self::from_code(s).ok_or_else(|| format!("Invalid country code: {}", s))
            }
        }

        impl ::core::convert::TryFrom<&str> for #name {
            type Error = ::std::string::String;

            fn try_from(s: &str) -> ::core::result::Result<Self, Self::Error> {
                Self::from_code(s).ok_or_else(|| format!("Invalid country code: {}", s))
            }
        }
    }
    .into()
}
