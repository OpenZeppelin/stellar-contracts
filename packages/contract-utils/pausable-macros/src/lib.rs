use proc_macro::TokenStream;
use stellar_macro_helpers::{parse_env_arg, FunctionInsert};

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
pub fn when_not_paused(attrs: TokenStream, item: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");
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
pub fn when_paused(attrs: TokenStream, item: TokenStream) -> TokenStream {
    assert!(attrs.is_empty(), "This macro does not accept any arguments");
    generate_pause_check(item, "when_paused")
}

fn generate_pause_check(input: TokenStream, check_fn: &str) -> TokenStream {
    let Ok(syn::Item::Fn(mut input_fn)): Result<syn::Item, syn::Error> =
        syn::parse::<syn::Item>(input.clone())
    else {
        return input;
    };
    let env_arg = parse_env_arg(&input_fn);
    let check_ident = syn::Ident::new(check_fn, proc_macro2::Span::call_site());
    input_fn
        .insert_stmts_to_token_stream(syn::parse_quote! {
            Self::#check_ident(#env_arg);
        })
        .into()
}
