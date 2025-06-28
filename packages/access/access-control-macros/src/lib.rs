use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::{generate_auth_check, parse_env_arg};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, Ident, ItemFn, LitStr, Pat, Token, Type,
};

/// A procedural macro that retrieves the admin from storage and requires
/// authorization from the admin before executing the function body.
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
pub fn only_admin(attrs: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");

    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate the function with the admin authorization check
    let auth_check_path = quote! { stellar_access_control::enforce_admin_auth };
    let expanded = generate_auth_check(&input_fn, auth_check_path);

    TokenStream::from(expanded)
}

/// A procedural macro that ensures the parameter has the specified role.
///
/// # Security Warning
///
/// **IMPORTANT**: This macro does NOT enforce authorization. This is a
/// deliberate choice, since in Stellar contracts, duplicate `require_auth()`
/// calls for the same account within the same call stack panics. This macro is
/// designed for use cases where you want to further limit a function that has
/// `require_auth()` in it with access control roles. If you also need
/// `require_auth()` provided by the macro, please use `#[only_role]` instead.
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
    generate_role_check(args, input, false)
}

/// A procedural macro that ensures the parameter has the specified role and
/// requires authorization.
///
/// **IMPORTANT**: This macro does enforce authorization. This is a deliberate
/// choice, since in Stellar contracts, duplicate `require_auth()` calls for the
/// same account within the same call stack panics. If you are getting errors
/// while using this macro, it could be that the function you're annotating
/// already has a `require_auth()` call for the same account. In that case,
/// please use `#[has_role]` instead.
///
/// # Usage
///
/// ```rust
/// #[only_role(account, "minter")]
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
///     account.require_auth();
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_role(args: TokenStream, input: TokenStream) -> TokenStream {
    generate_role_check(args, input, true)
}

/// Helper function that generates the role check code for both has_role and
/// only_role macros. If require_auth is true, it also adds the
/// account.require_auth() call.
fn generate_role_check(args: TokenStream, input: TokenStream, require_auth: bool) -> TokenStream {
    let args = parse_macro_input!(args as HasRoleArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let param_name = args.param;
    let role_str = args.role;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

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

    let auth_check = if require_auth {
        quote! { #param_name.require_auth(); }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            stellar_access_control::ensure_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, #role_str));
            #auth_check
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

struct HasAnyRoleArgs {
    param: Ident,
    roles: Vec<LitStr>,
}

impl Parse for HasAnyRoleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let param: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut roles = Vec::new();
        let first_role: LitStr = input.parse()?;
        roles.push(first_role);

        // Parse additional roles separated by commas
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            let role: LitStr = input.parse()?;
            roles.push(role);
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

/// A procedural macro that ensures the parameter has any of the specified
/// roles.
///
/// # Security Warning
///
/// **IMPORTANT**: This macro does NOT enforce authorization. This is a
/// deliberate choice, since in Stellar contracts, duplicate `require_auth()`
/// calls for the same account within the same call stack panics. This macro is
/// designed for use cases where you want to further limit a function that has
/// `require_auth()` in it with access control roles.
///
/// # Usage
///
/// ```rust
/// #[has_any_role(account, "minter", "admin", "operator")]
/// pub fn manage_tokens(e: &Env, amount: u32, account: Address) {
///     // Function body
/// }
/// ```
///
/// This will expand to code that checks if the account has any of the specified
/// roles.
#[proc_macro_attribute]
pub fn has_any_role(args: TokenStream, input: TokenStream) -> TokenStream {
    generate_any_role_check(args, input, false)
}

/// A procedural macro that ensures the parameter has any of the specified roles
/// and requires authorization.
///
/// **IMPORTANT**: This macro does enforce authorization. This is a deliberate
/// choice, since in Stellar contracts, duplicate `require_auth()` calls for the
/// same account within the same call stack panics. If you are getting errors
/// while using this macro, it could be that the function you're annotating
/// already has a `require_auth()` call for the same account. In that case,
/// please use `#[has_any_role]` instead.
///
/// # Usage
///
/// ```rust
/// #[only_any_role(account, "minter", "admin", "operator")]
/// pub fn manage_tokens(e: &Env, amount: u32, account: Address) {
///     // Function body
/// }
/// ```
///
/// This will expand to code that checks if the account has any of the specified
/// roles and requires authorization from the account.
#[proc_macro_attribute]
pub fn only_any_role(args: TokenStream, input: TokenStream) -> TokenStream {
    generate_any_role_check(args, input, true)
}

/// Helper function that generates the role check code for the has_any_role
/// macro. If require_auth is true, it also adds the account.require_auth()
/// call.
fn generate_any_role_check(
    args: TokenStream,
    input: TokenStream,
    require_auth: bool,
) -> TokenStream {
    let args = parse_macro_input!(args as HasAnyRoleArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let param_name = args.param;
    let roles = args.roles;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

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

    let auth_check = if require_auth {
        quote! { #param_name.require_auth(); }
    } else {
        quote! {}
    };

    let combined_checks = quote! {
        let has_any_role = [#(#roles),*].iter().any(|role| stellar_access_control::has_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, role)).is_some());
        if !has_any_role {
            panic!("Account does not have any of the required roles");
        }
    };

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            #combined_checks
            #auth_check
            #fn_block
        }
    };

    TokenStream::from(expanded)
}
