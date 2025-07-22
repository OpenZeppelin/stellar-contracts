use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::{add_auth_check, parse_env_arg, FunctionInsert};
use syn::{
    bracketed,
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
    let auth_check_path = quote! { Self::enforce_admin_auth };
    add_auth_check(parse_macro_input!(input as syn::Item), auth_check_path).into()
}

/// A procedural macro that ensures the parameter has the specified role.
///
/// # Security Warning
///
/// **IMPORTANT**: This macro checks role membership but does NOT enforce
/// authorization. This design prevents duplicate `require_auth()` calls which
/// would cause panics in Stellar contracts. Use this macro when:
///
/// 1. Your function already contains a `require_auth()` call
/// 2. You need additional role-based access control
///
/// If you need both role checking AND authorization, use `#[only_role]`
/// instead.
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
/// **IMPORTANT**: This macro both checks role membership AND enforces
/// authorization. Be aware that in Stellar contracts, duplicate
/// `require_auth()` calls for the same account will cause panics. If your
/// function already contains a `require_auth()` call for the same account, use
/// `#[has_role]` instead to avoid duplicate authorization checks.
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
    let Ok(syn::Item::Fn(mut input_fn)): Result<syn::Item, syn::Error> =
        syn::parse::<syn::Item>(input.clone())
    else {
        return input;
    };

    let env_arg = parse_env_arg(&input_fn);
    let args = parse_macro_input!(args as HasRoleArgs);

    let param_name = args.param;
    let role_str = args.role;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let auth_check = require_auth.then(|| quote! { #param_name.require_auth(); });

    input_fn.insert_stmts_to_token_stream(syn::parse_quote! {
        Self::ensure_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, #role_str));
        #auth_check
    }).into()
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

/// A procedural macro that ensures the parameter has any of the specified
/// roles.
///
/// # Security Warning
///
/// **IMPORTANT**: This macro checks role membership but does NOT enforce
/// authorization. This design prevents duplicate `require_auth()` calls which
/// would cause panics in Stellar contracts. Use this macro when:
///
/// 1. Your function already contains a `require_auth()` call
/// 2. You need additional role-based access control
///
/// If you need both role checking AND authorization, use `#[only_any_role]`
/// instead.
///
/// # Usage
///
/// ```rust
/// #[has_any_role(account, ["minter", "admin", "operator"])]
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
/// **IMPORTANT**: This macro both checks role membership AND enforces
/// authorization. Be aware that in Stellar contracts, duplicate
/// `require_auth()` calls for the same account will cause panics. If your
/// function already contains a `require_auth()` call for the same account, use
/// `#[has_any_role]` instead to avoid duplicate authorization checks.
///
/// # Usage
///
/// ```rust
/// #[only_any_role(account, ["minter", "admin", "operator"])]
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
    let Ok(syn::Item::Fn(mut input_fn)): Result<syn::Item, syn::Error> =
        syn::parse::<syn::Item>(input.clone())
    else {
        return input;
    };
    let env_arg = parse_env_arg(&input_fn);
    let param_name = args.param;
    let roles = args.roles;

    let is_ref_param = validate_address_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let auth_check = require_auth.then(|| quote! { #param_name.require_auth(); });

    input_fn.insert_stmts_to_token_stream(syn::parse_quote! {
        let has_any_role = [#(#roles),*].iter().any(|role| <Self as AccessControl>::has_role(#env_arg, #param_reference, &soroban_sdk::Symbol::new(#env_arg, role)).is_some());
        if !has_any_role {
            panic!("Account does not have any of the required roles");
        }
        #auth_check
    }).into()
}
