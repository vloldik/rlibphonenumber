mod helper_functions;
mod macro_input;

use helper_functions::{apply_mapper, match_type, substitute_bindings};
use macro_input::{PublicWrapperInput, Rule};
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{FnArg, ImplItem, ItemImpl, ReturnType, Type, parse_macro_input};

pub fn wrap_util(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = parse_macro_input!(attr as PublicWrapperInput);
    let mut input_impl = parse_macro_input!(item as ItemImpl);

    let (impl_generics, ty_generics, where_clause) = input_impl.generics.split_for_impl();

    let original_struct = &input_impl.self_ty;

    let mut generated_structs_and_impls = Vec::new();

    for wrapper in config.wrappers {
        let wrapper_name = &wrapper.name;
        let mut methods = Vec::new();

        for item in &mut input_impl.items {
            let ImplItem::Fn(method) = item else {
                continue;
            };
            let has_export = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("export"));

            if !has_export {
                continue;
            }
            let mut new_method = method.clone();
            new_method
                .attrs
                .retain(|attr| !attr.path().is_ident("export"));
            new_method.vis = syn::parse_quote!(pub);

            let sig = &mut new_method.sig;
            let method_name = &sig.ident;
            let mut args_call = Vec::new();

            for input in &mut sig.inputs {
                let FnArg::Typed(pat_type) = input else {
                    continue;
                };
                let arg_name = &pat_type.pat;
                let mut matched = false;

                for rule in &wrapper.rules {
                    if let Rule::Arg(arg_rule) = rule {
                        let mut bindings = HashMap::new();
                        let mut lifetime_bindings = HashMap::new();

                        if match_type(
                            &arg_rule.from,
                            &pat_type.ty,
                            &mut bindings,
                            &mut lifetime_bindings,
                        ) {
                            let mut new_ty = arg_rule.to.clone();

                            substitute_bindings(&mut new_ty, &bindings, &lifetime_bindings);
                            *pat_type.ty = new_ty;

                            let mapped_arg = apply_mapper(&arg_rule.mapper, &quote!(#arg_name));
                            args_call.push(mapped_arg);
                            matched = true;
                            break;
                        }
                    }
                }
                if !matched {
                    args_call.push(quote!(#arg_name));
                }
            }

            let mut self_call = if sig.receiver().is_some() {
                quote! { self.inner.#method_name(#(#args_call),*) }
            } else {
                let Type::Path(path) = original_struct.as_ref() else {
                    panic!("Invalid receiver: {}", quote! { #original_struct })
                };

                let seg = path.path.segments.last().map(|path| &path.ident);

                quote! { #seg::#method_name(#(#args_call),*) }
            };

            if let ReturnType::Type(_, ty) = &mut sig.output {
                for rule in &wrapper.rules {
                    if let Rule::Ret(ret_rule) = rule {
                        let mut bindings = HashMap::new();
                        let mut lifetime_bindings = HashMap::new();

                        if match_type(&ret_rule.from, ty, &mut bindings, &mut lifetime_bindings) {
                            let mut new_ty = ret_rule.to.clone();
                            substitute_bindings(&mut new_ty, &bindings, &lifetime_bindings);
                            **ty = new_ty;

                            self_call = apply_mapper(&ret_rule.mapper, &self_call);
                            break;
                        }
                    }
                }
            }

            new_method.block = syn::parse_quote!({ #self_call });
            methods.push(new_method);
        }

        generated_structs_and_impls.push(quote! {
            pub struct #wrapper_name #impl_generics #where_clause {
                pub(crate) inner: #original_struct,
            }

            impl #impl_generics #wrapper_name #ty_generics #where_clause {
                #(#methods)*
            }
        });
    }

    let expanded = quote! {
        #input_impl
        #(#generated_structs_and_impls)*
    };

    TokenStream::from(expanded)
}
