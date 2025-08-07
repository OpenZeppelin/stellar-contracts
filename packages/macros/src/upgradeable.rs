use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// Procedural macro implementation for `#[derive(Upgradeable)]`.
///
/// This function generates the implementation of the `Upgradeable` trait for a
/// given contract type, enabling the contract to be upgraded by replacing its
/// WASM bytecode.
///
/// # Behavior
///
/// - Implements the `upgrade` function with access control (`_require_auth`).
/// - Sets the contract crate version  as `"binver"` metadata using
///   `soroban_sdk::contractmeta!`. Gets the crate version via the env variable
///   `CARGO_PKG_VERSION` which corresponds to the "version" attribute in
///   Cargo.toml. If no such attribute or if it is "0.0.0", skips this step.
/// - Throws a compile-time error if `UpgradeableInternal` is not implemented.
///
/// # Example
/// ```ignore,rust
/// #[derive(Upgradeable)]
/// pub struct MyContract;
/// ```
pub fn derive_upgradeable(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let binver = set_binver_from_env();
    quote! {
        use stellar_access::Ownable;
        use stellar_contract_utils::upgradeable::Upgradable;

        #binver

        #[soroban_sdk::contractimpl]
        impl Ownable for #name {
            type Impl = stellar_access::Owner;
        }

        #[soroban_sdk::contractimpl]
        impl stellar_contract_utils::upgradeable::Upgradable for #name {}
    }
}

/// Sets the value of the environment variable `CARGO_PKG_VERSION` as `binver`
/// in the wasm binary metadata. This env variable corresponds to the attribute
/// "version" in Cargo.toml. If the attribute is missing or if it is "0.0.0",
/// the function does nothing.
fn set_binver_from_env() -> proc_macro2::TokenStream {
    // However when "version" is missing from Cargo.toml,
    // the following does not return error, but Ok("0.0.0")
    let version = std::env::var("CARGO_PKG_VERSION");

    match version {
        Ok(v) if v != "0.0.0" => {
            quote! { soroban_sdk::contractmeta!(key = "binver", val = #v); }
        }
        _ => quote! {},
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_set_binver_from_env_zero_version() {
        // Set version to 0.0.0
        env::set_var("CARGO_PKG_VERSION", "0.0.0");

        let result = set_binver_from_env();
        let result_str = result.to_string();

        // Should return empty tokens
        assert_eq!(result_str.trim(), "");
    }
}
