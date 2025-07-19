use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::add_auth_check;
use syn::{parse_macro_input, Item};

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
///     Self::enforce_owner_auth(e);
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_owner(attrs: TokenStream, input: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");
    // Generate the function with the owner authorization check
    let auth_check_path = quote! { Self::enforce_owner_auth };
    add_auth_check(parse_macro_input!(input as Item), auth_check_path).into()
}
