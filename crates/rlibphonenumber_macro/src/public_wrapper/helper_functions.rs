use quote::quote;
use std::collections::HashMap;
use syn::{
    Expr, PathArguments, Type,
    visit_mut::{VisitMut, visit_expr_mut},
};

pub fn apply_mapper(mapper: &Expr, target: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if let Expr::Closure(closure) = mapper {
        let mut param_ident = None;

        if let Some(pat) = closure.inputs.first() {
            match pat {
                syn::Pat::Ident(pat_ident) => param_ident = Some(pat_ident.ident.clone()),
                syn::Pat::Type(pat_type) => {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        param_ident = Some(pat_ident.ident.clone());
                    }
                }
                _ => {}
            }
        }

        if let Some(ident) = param_ident {
            let mut body = (*closure.body).clone();

            struct ParamReplacer<'a> {
                target: &'a proc_macro2::TokenStream,
                ident: &'a syn::Ident,
            }

            impl<'a> VisitMut for ParamReplacer<'a> {
                fn visit_expr_mut(&mut self, i: &mut Expr) {
                    if let Expr::Path(p) = i
                        && p.path.is_ident(self.ident)
                    {
                        let t = self.target;
                        *i = syn::parse_quote!((#t));
                        return;
                    }
                    visit_expr_mut(self, i);
                }
            }

            ParamReplacer {
                target,
                ident: &ident,
            }
            .visit_expr_mut(&mut body);
            return quote!(#body);
        }
    }

    quote!(#mapper(#target))
}

pub fn match_type(
    pat: &Type,
    actual: &Type,
    bindings: &mut HashMap<String, Type>,
    lifetime_bindings: &mut HashMap<String, syn::Lifetime>,
) -> bool {
    if let Type::Path(pat_path) = pat
        && pat_path.path.segments.len() == 1
    {
        let id_str = pat_path.path.segments[0].ident.to_string();
        if let Some(var_name) = id_str.strip_prefix("__Var_") {
            bindings.insert(var_name.to_string(), actual.clone());
            return true;
        }
    }

    match (pat, actual) {
        (Type::Path(pat_p), Type::Path(act_p)) => {
            let pat_seg = pat_p.path.segments.last().unwrap();
            let act_seg = act_p.path.segments.last().unwrap();

            if pat_seg.ident != act_seg.ident {
                return false;
            }

            match (&pat_seg.arguments, &act_seg.arguments) {
                (PathArguments::None, PathArguments::None) => true,
                (
                    PathArguments::AngleBracketed(pat_args),
                    PathArguments::AngleBracketed(act_args),
                ) => {
                    if pat_args.args.len() != act_args.args.len() {
                        return false;
                    }
                    for (p_arg, a_arg) in pat_args.args.iter().zip(act_args.args.iter()) {
                        match (p_arg, a_arg) {
                            (
                                syn::GenericArgument::Type(p_ty),
                                syn::GenericArgument::Type(a_ty),
                            ) => {
                                if !match_type(p_ty, a_ty, bindings, lifetime_bindings) {
                                    return false;
                                }
                            }
                            (
                                syn::GenericArgument::Lifetime(p_lt),
                                syn::GenericArgument::Lifetime(a_lt),
                            ) => {
                                let p_id = p_lt.ident.to_string();
                                if p_id != "static" {
                                    lifetime_bindings.insert(p_id, a_lt.clone());
                                } else if a_lt.ident != "static" {
                                    return false;
                                }
                            }
                            _ => {}
                        }
                    }
                    true
                }
                _ => false,
            }
        }
        (Type::Reference(pat_r), Type::Reference(act_r)) => {
            if pat_r.mutability.is_some() != act_r.mutability.is_some() {
                return false;
            }
            if let Some(p_lt) = &pat_r.lifetime
                && let Some(r_lt) = &act_r.lifetime
            {
                if p_lt.ident == "static" && r_lt.ident != "static" {
                    return false;
                }
                lifetime_bindings.insert(p_lt.ident.to_string(), r_lt.clone());
            } else if !(pat_r.lifetime.as_ref().is_none_or(|lt| lt.ident == "_")
                && act_r.lifetime.as_ref().is_none_or(|lt| lt.ident == "_"))
            {
                return false;
            }

            match_type(&pat_r.elem, &act_r.elem, bindings, lifetime_bindings)
        }
        (Type::Tuple(pat_t), Type::Tuple(act_t)) => {
            if pat_t.elems.len() != act_t.elems.len() {
                return false;
            }
            for (p, a) in pat_t.elems.iter().zip(act_t.elems.iter()) {
                if !match_type(p, a, bindings, lifetime_bindings) {
                    return false;
                }
            }
            true
        }
        (Type::Array(pat_arr), Type::Array(act_arr)) => {
            match_type(&pat_arr.elem, &act_arr.elem, bindings, lifetime_bindings)
        }
        (p, a) => {
            quote!(#p).to_string().replace(" ", "") == quote!(#a).to_string().replace(" ", "")
        }
    }
}

pub fn substitute_bindings(
    ty: &mut Type,
    bindings: &HashMap<String, Type>,
    lifetime_bindings: &HashMap<String, syn::Lifetime>,
) {
    struct Substituter<'a> {
        bindings: &'a HashMap<String, Type>,
        lifetime_bindings: &'a HashMap<String, syn::Lifetime>,
    }

    impl<'a> VisitMut for Substituter<'a> {
        fn visit_type_mut(&mut self, i: &mut Type) {
            if let Type::Path(p) = i
                && p.path.segments.len() == 1
            {
                let id_str = p.path.segments[0].ident.to_string();
                if let Some(var_name) = id_str.strip_prefix("__Var_")
                    && let Some(bound_ty) = self.bindings.get(var_name)
                {
                    *i = bound_ty.clone();
                    return;
                }
            }
            syn::visit_mut::visit_type_mut(self, i);
        }

        fn visit_lifetime_mut(&mut self, i: &mut syn::Lifetime) {
            if let Some(bound_lt) = self.lifetime_bindings.get(&i.ident.to_string()) {
                *i = bound_lt.clone();
            }
            syn::visit_mut::visit_lifetime_mut(self, i);
        }
    }

    Substituter {
        bindings,
        lifetime_bindings,
    }
    .visit_type_mut(ty);
}

#[cfg(test)]
mod test {
    use crate::public_wrapper::helper_functions::match_type;

    #[test]
    fn test() {
        use syn::parse_quote;

        assert!(match_type(
            &parse_quote!(&'_ str),
            &parse_quote!(&str),
            &mut Default::default(),
            &mut Default::default()
        ))
    }
}
