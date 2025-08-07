//! This crate is a collection of utility functions for stellar related macros.
//! It is not intended to be used directly, but rather imported into other
//! macros.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{FnArg, Ident, ItemFn, Pat, PatType, Type, TypePath};

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

pub fn insert_check(input_fn: syn::Item, auth_check_func: TokenStream) -> TokenStream {
    let syn::Item::Fn(mut input_fn) = input_fn else { return input_fn.to_token_stream() };
    // Get the environment parameter
    let env_param = parse_env_arg(&input_fn);

    input_fn.insert_stmts_to_token_stream(syn::parse_quote! {
        #auth_check_func(#env_param);
    })
}

pub trait FunctionInsert: ToTokens {
    fn insert_stmts(&mut self, stmts: Vec<syn::Stmt>);

    fn insert_stmts_to_token_stream(&mut self, stmts: Vec<syn::Stmt>) -> TokenStream {
        self.insert_stmts(stmts);
        self.to_token_stream()
    }
}

impl FunctionInsert for ItemFn {
    fn insert_stmts(&mut self, stmts: Vec<syn::Stmt>) {
        self.block.stmts.splice(0..0, stmts);
    }
}

#[cfg(test)]
mod test {
    use syn::parse_quote;

    use super::*;
    use crate::{generate_any_role_check, HasAnyRoleArgs};

    #[test]
    fn only_admin() {
        let auth_check_func = quote! { Self::enforce_admin_auth };
        let input_fn = parse_quote! {
            fn my_function(e: &Env) {
                my_code();
            }
        };

        let result = insert_check(input_fn, auth_check_func);
        assert_eq!(
            result.to_string(),
            quote! {
                fn my_function(e: &Env) {
                    Self::enforce_admin_auth(e);
                    my_code();
                }
            }
            .to_string()
        );
    }

    #[test]
    fn test_insert_check() {
        let auth_check_func = quote! { auth_check };
        let input_fn = parse_quote! {
            pub fn my_function(env: &Env) {
                my_code();
            }
        };
        let result = insert_check(input_fn, auth_check_func);
        assert_eq!(
            result.to_string(),
            quote! {
                pub fn my_function(env: &Env) {
                    auth_check(env);
                    my_code();
                }
            }
            .to_string()
        );
    }

    #[test]
    fn has_any_role() {
        let args = HasAnyRoleArgs {
            param: parse_quote!(caller),
            roles: vec![parse_quote!("admin"), parse_quote!("user")],
        };
        let input_fn = parse_quote! {
            pub fn multi_role_action(e: &Env, caller: Address) -> String {
                caller.require_auth();
                String::from_str(e, "multi_role_action_success")
            }
        };
        let result = generate_any_role_check(args, input_fn, false);
        assert_eq!(
            result.to_string(),
            quote! {
                pub fn multi_role_action(e: &Env, caller: Address) -> String {
                    Self::assert_has_any_role(e, &caller, &["admin", "user"]);
                    caller.require_auth();
                    String::from_str(e, "multi_role_action_success")
                }
            }
            .to_string()
        );
    }
}
