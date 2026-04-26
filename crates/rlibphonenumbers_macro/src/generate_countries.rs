use std::collections::{HashMap, HashSet};

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

    let mut countries_data = HashMap::new();
    for record in rdr.records().flatten() {
        if record.len() >= 2 {
            let name = record[0].trim().to_string();
            let code = record[1].trim().to_uppercase();
            if code.len() == 2 {
                countries_data.insert(code, name);
            }
        }
    }

    let missing_codes: Vec<_> = metadata_countries
        .iter()
        .filter(|code| !countries_data.contains_key(*code))
        .collect();

    for code in missing_codes {
        countries_data.insert(
            code.clone(),
            format!(
                "Exceptionally reserved / Libphonenumber specific region ({})",
                code
            ),
        );
    }

    // 676 Iterations
    for letter_a in b'A'..=b'Z' {
        for letter_b in b'A'..=b'Z' {
            let arr = [letter_a, letter_b];

            // A-Z, A-Z are guaranteed to contain only valid characters
            let code = str::from_utf8(&arr).unwrap();
            if !countries_data.contains_key(code) {
                countries_data.insert(code.to_string(), "Reserved not used code".into());
            }
        }
    }

    let mut countries_data = countries_data
        .into_iter()
        .map(|(code, name)| CountryData { name, code })
        .collect::<Vec<_>>();

    countries_data.sort_by(|a, b| a.code.cmp(&b.code));

    let string_to_code_val = |s: &str| -> u16 {
        let bytes = s.as_bytes();
        let first = bytes[0].to_ascii_uppercase() as u16;
        let second = bytes[1].to_ascii_uppercase() as u16;
        (first << 8) | second
    };

    let variants = countries_data.iter().map(|country| {
        let ident = format_ident!("{}", country.code);
        let name_str = &country.name;

        let is_presented = if metadata_countries.contains(&country.code) {
            "YES"
        } else {
            "NO"
        };
        let doc_comment = format!("{}\n\npresented - **{}**", name_str, is_presented);
        let str_code = string_to_code_val(&country.code);

        quote! {
            #[doc = #doc_comment]
            #ident = #str_code
        }
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
        #[repr(u16)]
        pub enum #name {
            #[doc = "001 - Non-Geographical Entity (World / Global Network)"]
            World = 1,
            #(#variants),*
        }

        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
            ::core::cmp::PartialOrd,
            ::core::cmp::Ord
        )]
        pub struct RegionStr {
            len: u8,
            buf: [u8; 3],
        }

        impl RegionStr {
            #[inline]
            pub fn as_str(&self) -> &str {
                // SAFETY: buffer is validated while creation (ASCII-only)
                unsafe { ::core::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
            }
        }

        impl ::core::ops::Deref for RegionStr {
            type Target = str;
            #[inline]
            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl ::core::convert::AsRef<str> for RegionStr {
            #[inline]
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl ::core::fmt::Display for RegionStr {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::core::fmt::Debug for RegionStr {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Debug::fmt(self.as_str(), f)
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum InvalidRegionError {
            InvalidCharacter([u8; 2]),
            InvalidLength(usize),
        }

        impl ::core::fmt::Display for InvalidRegionError {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    InvalidRegionError::InvalidCharacter(c) => {
                        write!(
                            f, 
                            "Invalid characters in region code, only ASCII letters expected. Got: '{}' and '{}'", 
                            ::core::ascii::escape_default(c[0]), 
                            ::core::ascii::escape_default(c[1])
                        )
                    }
                    InvalidRegionError::InvalidLength(len) => {
                        write!(f, "Exactly two letters expected (or '001'), got length {}", len)
                    }
                }
            }
        }
        impl ::std::error::Error for InvalidRegionError {}

        impl #name {
            #[inline]
            const fn extract_letters(bytes: &[u8]) -> ::core::result::Result<(u8, u8), InvalidRegionError>  {
                if bytes.len() != 2 {
                    return ::core::result::Result::Err(InvalidRegionError::InvalidLength(bytes.len()));
                }
                if !bytes[0].is_ascii_alphabetic() || !bytes[1].is_ascii_alphabetic() {
                    return ::core::result::Result::Err(InvalidRegionError::InvalidCharacter([bytes[0], bytes[1]]));
                }

                let first = bytes[0].to_ascii_uppercase();
                let second = bytes[1].to_ascii_uppercase();

                ::core::result::Result::Ok((first, second))
            }

            pub fn from_code(s: &str) -> ::core::result::Result<Self, InvalidRegionError> {
                if s == "001" {
                    return ::core::result::Result::Ok(Self::World);
                }
                let (first, second) = Self::extract_letters(s.as_bytes())?;
                let code = ((first as u16) << 8) | (second as u16);

                ::core::result::Result::Ok(
                    // SAFETY: `extract_letters` guarantees that bytes are within A-Z (0x41-0x5A).
                    // The macro generates exactly 676 variants covering all combinations in this range.
                    // World (1) is handled separately. Therefore, `code` perfectly maps to a valid discriminant.
                    unsafe { ::core::mem::transmute(code) }
                )
            }

            pub const fn as_region_str(&self) -> RegionStr {
                let mut buf = [0; 3];
                let len;
                match self {
                    Self::World => {
                        buf[0] = b'0'; buf[1] = b'0'; buf[2] = b'1';
                        len = 3;
                    }
                    _ => {
                        let val = *self as u16;
                        buf[0] = (val >> 8) as u8;
                        buf[1] = val as u8;
                        len = 2;
                    }
                }
                RegionStr { len, buf }
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&*self.as_region_str())
            }
        }

        impl ::core::str::FromStr for #name {
            type Err = InvalidRegionError;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                Self::from_code(s)
            }
        }

        impl ::core::convert::TryFrom<&str> for #name {
            type Error = InvalidRegionError;

            fn try_from(s: &str) -> ::core::result::Result<Self, Self::Error> {
                Self::from_code(s)
            }
        }
    }
    .into()
}
