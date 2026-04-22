use proc_macro::TokenStream;
use quote::{format_ident, quote};
use xml::{EventReader, reader::XmlEvent};

fn get_country_list() -> Vec<String> {
    let file = include_str!(concat!(env!("OUT_DIR"), "/PhoneNumberMetadata.xml"));
    let mut countries = Vec::new();

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
            countries.push(value.to_string());
        }
    }

    countries
}

#[proc_macro]
pub fn countries_enum(name: TokenStream) -> TokenStream {
    let country_list = get_country_list();
    let countries = country_list
        .iter()
        .map(|country| format_ident!("{}", country.to_uppercase()));

    let str_mapper = country_list.iter().map(|country| {
        let item = format_ident!("{}", country.to_uppercase());
        let value = proc_macro2::Literal::string(country);

        quote! { Self::#item => #value }
    });

    let from_str_mapper = country_list.iter().map(|country| {
        let item = format_ident!("{}", country.to_uppercase());
        let value = proc_macro2::Literal::string(country);
        quote! { #value => ::core::result::Result::Ok(Self::#item) }
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
            #(#countries),*
        }

        impl ::core::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
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
            type Err = String;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_mapper,)*
                    _ => ::core::result::Result::Err(format!("Unknown country code: {}", s)),
                }
            }
        }
    }
    .into()
}
