use proc_macro::TokenStream;

use crate::helper::generate_pause_check;

mod helper;

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
/// pub fn my_function(env: &Env) {
///     // This code will only execute if the contract is not paused
/// }
/// ```
#[proc_macro_attribute]
pub fn when_not_paused(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_pause_check(item, "when_not_paused")
}

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
/// pub fn my_function(env: &Env) {
///     // This code will only execute if the contract is paused
/// }
/// ```
#[proc_macro_attribute]
pub fn when_paused(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_pause_check(item, "when_paused")
}
