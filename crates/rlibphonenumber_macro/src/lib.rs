use proc_macro::TokenStream;

mod generate_countries;
mod public_wrapper;

#[proc_macro]
pub fn countries_enum(name: TokenStream) -> TokenStream {
    generate_countries::countries_enum(name)
}

#[proc_macro_attribute]
pub fn public_wrapper(attr: TokenStream, item: TokenStream) -> TokenStream {
    public_wrapper::wrap_util(attr, item)
}

#[proc_macro_attribute]
pub fn export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
