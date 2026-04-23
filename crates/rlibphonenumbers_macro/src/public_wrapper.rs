use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ItemImpl, Pat, PathArguments, ReturnType, Signature, Type, TypePath, parse_macro_input, parse_quote, };

pub fn wrap_util(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_impl = parse_macro_input!(item as ItemImpl);
    let original_struct = &input_impl.self_ty;
    let orig_name = format_ident!("{}", quote!(#original_struct).to_string().replace("Internal", ""));

    let wrapper_a = format_ident!("{}", orig_name);
    let wrapper_b = format_ident!("{}Fallible", orig_name);

    let mut methods_a = Vec::new();
    let mut methods_b = Vec::new();

    for item in &mut input_impl.items {
        if let ImplItem::Fn(method) = item {
            let has_export = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("export"));
            let is_pub = matches!(method.vis, syn::Visibility::Public(_));

            if has_export || is_pub {
                method.attrs.retain(|attr| !attr.path().is_ident("export"));

                let sig = &method.sig;
                let method_name = &sig.ident;
                let (sig, args_call) = transform_sig_for_as_ref(sig);
                let self_call = if sig.receiver().is_some() {
                    quote! { self.inner.#method_name(#(#args_call),*) }
                } else {
                    quote! { #original_struct::#method_name(#(#args_call),*) }
                };
                let (self_call_fallible, self_call_infallible) = (
                    transform_call(self_call.clone(), sig.clone(), false),
                    transform_call(self_call.clone(), sig.clone(), true),
                );

                methods_a.push(self_call_infallible);
                methods_b.push(self_call_fallible);
            }
        }
    }

    let expanded = quote! {
        #input_impl

        pub struct #wrapper_a {
            pub inner: #original_struct,
        }

        impl #wrapper_a {
            #(#methods_a)*
        }

        pub struct #wrapper_b {
            pub inner: #original_struct,
        }

        impl #wrapper_b {
            #(#methods_b)*
        }
    };

    TokenStream::from(expanded)
}

fn transform_sig_for_as_ref(sig: &Signature) -> (Signature, Vec<proc_macro2::TokenStream>) {
    let mut new_sig = sig.clone();
    let mut args_call = Vec::new();
    let mut generic_counter: usize = 0;

    for input in &mut new_sig.inputs {
        match input {
            FnArg::Receiver(_) => {}
            FnArg::Typed(pat_type) => {
                let is_str = if let Type::Reference(r) = &*pat_type.ty {
                    if let Type::Path(p) = r.elem.as_ref() {
                        p.path.is_ident("str")
                    } else {
                        false
                    }
                } else {
                    false
                };

                let arg_name = if let Pat::Ident(id) = pat_type.pat.as_ref() {
                    &id.ident
                } else {
                    panic!("Unsupported pattern");
                };

                if is_str {
                    generic_counter += 1;
                    let gen_ident = format_ident!("S{}", generic_counter);
                    *pat_type.ty = syn::parse_quote!(#gen_ident);
                    new_sig
                        .generics
                        .params
                        .push(syn::parse_quote!(#gen_ident: AsRef<str>));

                    args_call.push(quote! { #arg_name.as_ref() });
                } else {
                    args_call.push(quote! { #arg_name });
                }
            }
        }
    }

    (new_sig, args_call)
}

fn transform_return_value_path(path: &TypePath, self_call: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if path.path.is_ident("Self") {
        return quote! { Self { inner: #self_call } };
    }
    let Some(last_segment) = path.path.segments.last().filter(| last | last.ident == "Result") else {
        return self_call;
    };

    if let PathArguments::AngleBracketed(_) = &last_segment.arguments {
        quote! { Ok( Self { inner: #self_call? } ) }
    } else {
        self_call
    }
}

fn transform_call(self_call: proc_macro2::TokenStream, mut sig: Signature, for_public: bool) -> proc_macro2::TokenStream {
    match sig.output.clone() {
        ReturnType::Default => self_call,
        ReturnType::Type(_arr, type_) => {
            match type_.as_ref() {
                Type::Path(type_path) if !for_public => {
                    let call = transform_return_value_path(type_path, self_call);
                    quote! {
                        pub #sig {
                            #call
                        }
                    }
                },
                Type::Path(type_path) if for_public => {
                    let call = transform_for_public(type_path, &mut sig, transform_return_value_path(type_path, self_call));
                    quote! {
                        pub #sig {
                            #call
                        }
                    }
                },
                _ => self_call,
            }
        }
    }
}

fn transform_for_public(path: &TypePath, sig: &mut Signature, self_call: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let Some(last_segment) = path.path.segments.last()
       .filter(| last | last.ident == "ParseResult" || last.ident == "RegexResult") else {
        return self_call;
    };

    let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
        return self_call
    };

    let Some(return_type) = args.args.first() else {
        return self_call;
    };

    if last_segment.ident == "ParseResult" {
        sig.output = parse_quote!(-> ::core::result::Result<#return_type, ::crate::phonenumberutil::errors::ParseError>);
        quote! {
            #self_call
                .map_err(crate::phonenumberutil::errors::unwrap_internal)
        }
    } else {
        sig.output = parse_quote!( -> #return_type );
        quote! {
            #self_call
                .map_err(::crate::phonenumberutil::errors::unwrap_internal)
                .unwrap_or_else(| err | match err { })
        }
    }
}