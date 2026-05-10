use syn::{
    Expr, Token, Type,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct PublicWrapperInput {
    pub wrappers: Punctuated<WrapperDef, Token![,]>,
}

pub struct WrapperDef {
    pub name: syn::Ident,
    pub rules: Punctuated<Rule, Token![,]>,
}

pub enum Rule {
    Arg(Box<MapRule>),
    Ret(Box<MapRule>),
}

pub struct MapRule {
    pub from: Type,
    pub to: Type,
    pub mapper: Expr,
}

fn parse_pattern_type(input: ParseStream) -> syn::Result<Type> {
    let mut tokens = proc_macro2::TokenStream::new();
    while !input.peek(Token![->]) && !input.peek(Token![=>]) && !input.is_empty() {
        if input.peek(Token![$]) {
            let _dollar: Token![$] = input.parse()?;
            if let Ok(id) = syn::Ident::parse_any(input) {
                let var_ident = syn::Ident::new(&format!("__Var_{}", id), id.span());
                tokens.extend(quote::quote!(#var_ident));
            }
        } else {
            let tt: proc_macro2::TokenTree = input.parse()?;
            tokens.extend(std::iter::once(tt));
        }
    }
    syn::parse2(tokens)
}

impl Parse for PublicWrapperInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let wrappers = input.parse_terminated(WrapperDef::parse, Token![,])?;
        Ok(PublicWrapperInput { wrappers })
    }
}

impl Parse for WrapperDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = syn::Ident::parse_any(input)?;
        let content;
        syn::braced!(content in input);
        let rules = content.parse_terminated(Rule::parse, Token![,])?;
        Ok(WrapperDef { name, rules })
    }
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind = syn::Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;

        let from = parse_pattern_type(input)?;
        input.parse::<Token![->]>()?;
        let to = parse_pattern_type(input)?;
        input.parse::<Token![=>]>()?;
        let mapper: Expr = input.parse()?;

        let rule = Box::new(MapRule { from, to, mapper });

        if kind == "arg" {
            Ok(Rule::Arg(rule))
        } else if kind == "ret" {
            Ok(Rule::Ret(rule))
        } else {
            Err(syn::Error::new(kind.span(), "Expected `arg` or `ret`"))
        }
    }
}
