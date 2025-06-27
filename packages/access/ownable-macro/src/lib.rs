use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::generate_auth_check;
use syn::{parse_macro_input, ItemFn};

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
///         e.storage().instance().get(&stellar_ownable::OwnableStorageKey::Owner).unwrap();
///     owner.require_auth();
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_owner(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Generate the function with the owner authorization check
    let auth_check_path = quote! { stellar_ownable::enforce_owner_auth };
    let expanded = generate_auth_check(&input_fn, auth_check_path);

    TokenStream::from(expanded)
}
