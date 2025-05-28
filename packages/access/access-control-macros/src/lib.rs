use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::{generate_auth_check, parse_env_arg};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, Ident, ItemFn, LitStr, Pat, Token, Type,
};

/// A procedural macro that ensures the caller is the admin before executing the
/// function.
///
/// This macro retrieves the admin from storage and requires authorization from
/// the admin before executing the function body.
///
/// # Usage
///
/// ```rust
/// #[only_admin]
/// pub fn restricted_function(e: &Env, other_param: u32) {
///     // Function body
/// }
/// ```
///
/// This will expand to:
///
/// ```rust
/// pub fn restricted_function(e: &Env, other_param: u32) {
///     stellar_access_control::enforce_admin_auth(e);
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_admin(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate the function with the admin authorization check
    let auth_check_path = quote! { stellar_access_control::enforce_admin_auth };
    let expanded = generate_auth_check(&input_fn, auth_check_path);

    TokenStream::from(expanded)
}

/// A procedural macro that ensures the parameter has the specified role.
///
/// # Usage
///
/// ```rust
/// #[has_role(account, "minter")]
/// pub fn mint_tokens(e: &Env, amount: u32, account: Address) {
///     // Function body
/// }
/// ```
///
/// This will expand to:
///
/// ```rust
/// pub fn mint_tokens(e: &Env, amount: u32, account: Address) {
///     stellar_access_control::ensure_role(e, &account, &soroban_sdk::Symbol::new(e, "minter"));
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn has_role(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as HasRoleArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let param_name = args.param;
    let role_str = args.role;

    let is_ref_param = validate_param_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let fn_attrs = &input_fn.attrs;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let env_arg = parse_env_arg(&input_fn);

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            stellar_access_control::ensure_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, #role_str));
            #fn_block
        }
    };

    TokenStream::from(expanded)
}

struct HasRoleArgs {
    param: Ident,
    role: LitStr,
}

impl Parse for HasRoleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let param: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let role: LitStr = input.parse()?;
        Ok(HasRoleArgs { param, role })
    }
}

fn validate_param_type(func: &ItemFn, param_name: &Ident) -> bool {
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
