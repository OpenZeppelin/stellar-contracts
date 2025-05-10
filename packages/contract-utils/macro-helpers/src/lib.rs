//! This crate is a collection of utility functions for stellar related macros.
//! It is not intended to be used directly, but rather imported into other
//! macros.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Pat, PatIdent, PatType, Type, TypePath, TypeReference};

/// Parses the environment argument from the function signature
pub fn parse_env_arg(input_fn: &ItemFn) -> TokenStream {
    let (env_ident, is_ref) = check_env_arg(input_fn);

    if is_ref {
        quote! { #env_ident }
    } else {
        quote! { &#env_ident }
    }
}

fn check_env_arg(input_fn: &ItemFn) -> (Ident, bool) {
    // Get the first argument
    let first_arg = input_fn.sig.inputs.first().unwrap_or_else(|| {
        panic!("function '{}' must have at least one argument", input_fn.sig.ident)
    });

    // Extract the pattern and type from the argument
    let FnArg::Typed(PatType { pat, ty, .. }) = first_arg else {
        panic!("first argument of function '{}' must be a typed parameter", input_fn.sig.ident);
    };

    // Get the identifier from the pattern
    let Pat::Ident(pat_ident) = &**pat else {
        panic!("first argument of function '{}' must be an identifier", input_fn.sig.ident);
    };
    let ident = pat_ident.ident.clone();

    // Check if the type is Env or &Env
    let is_ref = match &**ty {
        Type::Reference(type_ref) => {
            let Type::Path(path) = &*type_ref.elem else {
                panic!("first argument of function '{}' must be Env or &Env", input_fn.sig.ident);
            };
            check_is_env(path, &input_fn.sig.ident);
            true
        }
        Type::Path(path) => {
            check_is_env(path, &input_fn.sig.ident);
            false
        }
        _ => panic!("first argument of function '{}' must be Env or &Env", input_fn.sig.ident),
    };

    (ident, is_ref)
}

fn check_is_env(path: &TypePath, fn_name: &Ident) {
    let is_env = path.path.segments.last().map(|seg| seg.ident == "Env").unwrap_or(false);

    if !is_env {
        panic!("first argument of function '{fn_name}' must be Env or &Env",);
    }
}

/// Find the first parameter of type Address or &Address
pub fn find_address_param(func: &ItemFn) -> Option<(proc_macro2::TokenStream, bool)> {
    for arg in &func.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                match &**ty {
                    // Check for &Address
                    Type::Reference(TypeReference { elem, .. }) => {
                        if is_address_type(elem) {
                            return Some((quote! { #ident }, true));
                        }
                    }
                    // Check for Address
                    Type::Path(_) => {
                        if is_address_type(ty) {
                            return Some((quote! { #ident }, false));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn is_address_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if path.segments.len() == 1 && path.segments[0].ident == "Address" {
            return true;
        }
    }
    false
}
