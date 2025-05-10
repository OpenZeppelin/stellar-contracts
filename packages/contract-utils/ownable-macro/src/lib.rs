use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::{find_address_param, parse_env_arg};
use syn::{parse_macro_input, ItemFn};

/// A procedural macro that ensures the caller is the owner before executing the function.
///
/// This macro finds the first parameter of type `Address` or `&Address` and uses it
/// as the caller parameter for the `ensure_is_owner` check.
///
/// # Usage
///
/// ```rust
/// #[only_owner]
/// pub fn restricted_function(e: &Env, caller: Address, other_param: u32) {
///     // Function body
/// }
/// ```
///
/// This will expand to:
///
/// ```rust
/// pub fn restricted_function(e: &Env, caller: Address, other_param: u32) {
///     ensure_is_owner(e, &caller);
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_owner(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Use the utility function to get the environment parameter
    let env_param = parse_env_arg(&input_fn);

    // Find the first Address parameter
    let (address_param, is_ref) = find_address_param(&input_fn)
        .expect("No parameter of type Address or &Address found in function signature");

    // Create the appropriate reference expression based on whether the parameter is already a reference
    let address_expr = if is_ref {
        quote! { #address_param }
    } else {
        quote! { &#address_param }
    };

    // Generate the function with the ensure_is_owner check
    let fn_attrs = &input_fn.attrs;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            stellar_ownable::ensure_is_owner(#env_param, #address_expr);
            #fn_block
        }
    };

    TokenStream::from(expanded)
}
