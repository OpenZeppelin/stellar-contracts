use proc_macro::TokenStream;
use quote::quote;
use stellar_macro_helpers::parse_env_arg;
use syn::{parse_macro_input, ItemFn};

/// A procedural macro that ensures the caller is the owner before executing the
/// function.
///
/// This macro retrieves the owner from storage and requires authorization from the owner
/// before executing the function body.
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
///     let owner: soroban_sdk::Address = e.storage().instance().get(&stellar_ownable::OwnableStorageKey::Owner).unwrap();
///     owner.require_auth();
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn only_owner(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // Use the utility function to get the environment parameter
    let env_param = parse_env_arg(&input_fn);

    // Generate the function with the owner retrieval and authorization check
    let fn_attrs = &input_fn.attrs;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            stellar_ownable::enforce_owner_auth(#env_param);
            #fn_block
        }
    };

    TokenStream::from(expanded)
}
