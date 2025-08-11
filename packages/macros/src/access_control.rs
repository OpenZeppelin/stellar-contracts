use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_quote, FnArg, Ident, ItemFn, LitStr, Pat, Token, Type,
};

use crate::{helpers::FunctionInsert, parse_env_arg};

/// Helper function that generates the role check code for both has_role and
/// only_role macros. If require_auth is true, it also adds the
/// account.require_auth() call.
pub fn generate_role_check(args: HasRoleArgs, input: syn::Item, require_auth: bool) -> TokenStream {
    let syn::Item::Fn(mut input_fn) = input.clone() else {
        return input.into_token_stream();
    };

    let param_name = args.param;
    let role_str = args.role;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let env_arg = parse_env_arg(&input_fn);

    let auth_check = require_auth.then(|| quote! { #param_name.require_auth(); });
    input_fn.insert_stmts_to_token_stream(parse_quote! {
            Self::ensure_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, #role_str));
            #auth_check
    })
}

pub struct HasRoleArgs {
    pub(crate) param: Ident,
    pub(crate) role: LitStr,
}

impl Parse for HasRoleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let param: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let role: LitStr = input.parse()?;
        Ok(HasRoleArgs { param, role })
    }
}

pub struct HasAnyRoleArgs {
    pub(crate) param: Ident,
    pub(crate) roles: Vec<LitStr>,
}

impl Parse for HasAnyRoleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the parameter name
        let param: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        // Parse the array of roles
        let content;
        bracketed!(content in input);

        let mut roles = Vec::new();

        // Parse roles until we reach the end of the array
        while !content.is_empty() {
            let role: LitStr = content.parse()?;
            roles.push(role);

            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }

        if roles.is_empty() {
            return Err(syn::Error::new(input.span(), "At least one role must be specified"));
        }

        Ok(HasAnyRoleArgs { param, roles })
    }
}

fn validate_address_type(func: &ItemFn, param_name: &Ident) -> bool {
    for arg in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.ident == *param_name {
                    return match_address_type(&pat_type.ty, param_name);
                }
            }
        }
    }
    panic!("Parameter `{param_name}` not found in function signature");
}

fn match_address_type(ty: &Type, param_name: &Ident) -> bool {
    match ty {
        Type::Reference(type_ref) => match_path_is_address(&type_ref.elem, param_name, true),
        Type::Path(_) => match_path_is_address(ty, param_name, false),
        _ => panic_type(param_name),
    }
}

fn match_path_is_address(ty: &Type, param_name: &Ident, is_ref: bool) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.last().map(|s| s.ident == "Address").unwrap_or(false) {
            return is_ref;
        }
    }
    panic_type(param_name);
}

fn panic_type(param_name: &Ident) -> ! {
    panic!("Parameter `{param_name}` must be of type `Address` or `&Address`");
}

/// Helper function that generates the role check code for the has_any_role
/// macro. If require_auth is true, it also adds the account.require_auth()
/// call.
pub fn generate_any_role_check(
    args: HasAnyRoleArgs,
    input: syn::Item,
    require_auth: bool,
) -> TokenStream {
    let syn::Item::Fn(mut input_fn) = input.clone() else {
        return input.into_token_stream();
    };

    let param_name = args.param;
    let roles = args.roles;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let env_arg = parse_env_arg(&input_fn);

    let auth_check = require_auth.then(|| {
        quote! { #param_name.require_auth(); }
    });

    input_fn.insert_stmts_to_token_stream(parse_quote! {
            Self::assert_has_any_role(#env_arg, #param_reference, &[#(#roles),*]);
            #auth_check
    })
}
