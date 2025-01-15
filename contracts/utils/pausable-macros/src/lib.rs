use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

use crate::helper::check_env_arg;

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
    let input_fn = parse_macro_input!(item as ItemFn);
    let (env_ident, is_ref) = check_env_arg(&input_fn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let env_arg = if is_ref {
        quote! { #env_ident }
    } else {
        quote! { &#env_ident }
    };

    let output = quote! {
        #fn_vis #fn_sig {
            openzeppelin_pausable::when_not_paused(#env_arg);

            #fn_block
        }
    };

    output.into()
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
    let input_fn = parse_macro_input!(item as ItemFn);
    let (env_ident, is_ref) = check_env_arg(&input_fn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let env_arg = if is_ref {
        quote! { #env_ident }
    } else {
        quote! { &#env_ident }
    };

    let output = quote! {
        #fn_vis #fn_sig {
            openzeppelin_pausable::when_paused(#env_arg);

            #fn_block
        }
    };

    output.into()
}
