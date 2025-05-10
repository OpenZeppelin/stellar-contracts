use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use stellar_macro_helpers::parse_env_arg;

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

fn generate_pause_check(item: TokenStream, check_fn: &str) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let env_arg = parse_env_arg(&input_fn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    let check_ident = syn::Ident::new(check_fn, proc_macro2::Span::call_site());
    let output = quote! {
        #(#fn_attrs)* // retain other macros
        #fn_vis #fn_sig {
            stellar_pausable::#check_ident(#env_arg);

            #fn_block
        }
    };

    output.into()
}
