mod access_control;
mod helpers;
mod pausable;

use access_control::{generate_any_role_check, generate_role_check};
use helpers::*;
use pausable::generate_pause_check;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/* ACCESS CONTROL MACROS */

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
///     stellar_access::access_control::enforce_admin_auth(e);
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_admin(attrs: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");

    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate the function with the admin authorization check
    let auth_check_path = quote! { stellar_access::access_control::enforce_admin_auth };
    let expanded = generate_auth_check(&input_fn, auth_check_path);

    TokenStream::from(expanded)
}

/// A procedural macro that ensures the parameter has the specified role.
///
/// # Security Warning
///
/// **IMPORTANT**: This macro checks role membership but does NOT enforce
/// authorization. This design prevents duplicate `require_auth()` calls which
/// would cause panics in Stellar contracts. Use this macro when:
///
/// 1. The function already contains a `require_auth()` call
/// 2. Additional role-based access control is needed
///
/// If both role checking AND authorization are needed, use `#[only_role]`
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
///     stellar_access::access_control::ensure_role(
///         e,
///         &account,
///         &soroban_sdk::Symbol::new(e, "minter"),
///     );
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
/// `require_auth()` calls for the same account will cause panics. If the
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
///     stellar_access::access_control::ensure_role(
///         e,
///         &account,
///         &soroban_sdk::Symbol::new(e, "minter"),
///     );
///     account.require_auth();
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_role(args: TokenStream, input: TokenStream) -> TokenStream {
    generate_role_check(args, input, true)
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
/// 1. The function already contains a `require_auth()` call
/// 2. Additional role-based access control is needed
///
/// If both role checking AND authorization are needed, use
/// `#[only_any_role]`
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
/// `require_auth()` calls for the same account will cause panics. If the
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

/// A procedural macro that retrieves the owner from storage and requires
/// authorization from the owner before executing the function body.
///
/// # Usage
///
/// ```rust
/// #[only_owner]
/// pub fn restricted_function(e: &Env, other_param: u32) {
///     // Function body
/// }
/// ```
///
/// This will expand to:
///
/// ```rust
/// pub fn restricted_function(e: &Env, other_param: u32) {
///     let owner: soroban_sdk::Address =
///         e.storage().instance().get(&stellar_access::ownable::OwnableStorageKey::Owner).unwrap();
///     owner.require_auth();
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_owner(attrs: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");

    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate the function with the owner authorization check
    let auth_check_path = quote! { stellar_access::ownable::enforce_owner_auth };
    let expanded = generate_auth_check(&input_fn, auth_check_path);

    TokenStream::from(expanded)
}

/// Adds a pause check at the beginning of the function that ensures the
/// contract is not paused.
///
/// This macro will inject a `when_not_paused` check at the start of the
/// function body. If the contract is paused, the function will return early
/// with a panic.
///
/// # Requirement:
///
/// - The first argument of the decorated function must be of type `Env` or
///   `&Env`
///
/// # Example:
///
/// ```ignore
/// #[when_not_paused]
/// pub fn my_function(e: &Env) {
///     // This code will only execute if the contract is not paused
/// }
/// ```
#[proc_macro_attribute]
pub fn when_not_paused(attrs: TokenStream, item: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");

    generate_pause_check(item, "when_not_paused")
}

/* PAUSABLE MACROS */

/// Adds a pause check at the beginning of the function that ensures the
/// contract is paused.
///
/// This macro will inject a `when_paused` check at the start of the function
/// body. If the contract is not paused, the function will return early with a
/// panic.
///
/// # Requirement:
///
/// - The first argument of the decorated function must be of type `Env` or
///   `&Env`
///
/// # Example:
///
/// ```ignore
/// #[when_paused]
/// pub fn my_function(e: &Env) {
///     // This code will only execute if the contract is paused
/// }
/// ```
#[proc_macro_attribute]
pub fn when_paused(attrs: TokenStream, item: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");

    generate_pause_check(item, "when_paused")
}
