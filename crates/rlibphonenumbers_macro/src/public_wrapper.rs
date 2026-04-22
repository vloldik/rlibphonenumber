use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{FnArg, ImplItem, ItemImpl, Pat, Signature, Type, parse_macro_input};

pub fn wrap_util(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_impl = parse_macro_input!(item as ItemImpl);
    let original_struct = &input_impl.self_ty;
    let orig_name = format_ident!("{}", quote!(#original_struct).to_string());

    let wrapper_a = format_ident!("{}Public", orig_name);
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
                let (new_sig, args_call) = transform_sig_for_as_ref(sig);
                let self_call = if new_sig.receiver().is_some() {
                    quote! { self.inner.#method_name }
                } else {
                    quote! { #orig_name::#method_name }
                };

                methods_a.push(quote! {
                    pub #new_sig {
                        #self_call(#(#args_call),*)
                    }
                });

                methods_b.push(quote! {
                    pub #new_sig {
                        #self_call(#(#args_call),*)
                    }
                });
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
                    if let Type::Path(p) = &*r.elem {
                        p.path.is_ident("str")
                    } else {
                        false
                    }
                } else {
                    false
                };

                let arg_name = if let Pat::Ident(id) = &*pat_type.pat {
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
