use proc_macro::TokenStream;
use quote::{format_ident, quote};
use xml::{EventReader, reader::XmlEvent};

fn get_country_list() -> Vec<String> {
    let file = include_str!("../../../resources/ShortNumberMetadata.xml");
    let mut countries = Vec::new();

    let parser = EventReader::from_str(file);
    for e in parser {
        if let Ok(XmlEvent::StartElement {
            name, attributes, ..
        }) = e
            && name.borrow().local_name == "territory"
        {
            countries.push(attributes[0].value.clone());
        }
    }

    countries
}

#[proc_macro]
pub fn contries_enum(name: TokenStream) -> TokenStream {
    let country_list = get_country_list();
    let countries = country_list
        .iter()
        .map(|country| format_ident!("{}", country.to_uppercase()));

    let str_mapper = country_list.iter().map(|country| {
        let item = format_ident!("{}", country.to_uppercase());
        let value = proc_macro2::Literal::string(country);

        quote! { Self::#item => #value }
    });

    let name = format_ident!("{}", name.to_string());

    quote! {
        #[derive(Debug)]
        enum #name {
            #(#countries),*
        }

        impl AsRef<str> for TEST {
            fn as_ref(&self) -> &str {
                match self {
                    #(#str_mapper),*
                }
            }
        }
    }
    .into()
}
