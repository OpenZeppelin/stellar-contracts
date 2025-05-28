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
        use stellar_upgradeable::Upgradeable as _;

        #binver

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::Upgradeable for #name {
            fn upgrade(
                e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>, operator: soroban_sdk::Address
            ) {
                Self::_require_auth(e, &operator);

                // Set the flag in case the next contract version needs to perform a migration
                // i.e. when the current version is `Upgradeable` only,
                // while the next one becomes `UpgradeableMigratable`.
                stellar_upgradeable::enable_migration(e);

                e.deployer().update_current_contract_wasm(new_wasm_hash);
            }
        }
    }
}

/// Procedural macro implementation for `#[derive(UpgradeableMigratable)]`.
///
/// This function generates the implementation of the `UpgradeableMigratable`
/// trait for a given contract type, wiring up the migration and rollback logic
/// based on the `UpgradeableMigratableInternal` trait implementation provided
/// by the user.
///
/// # Behavior
///
/// - Implements `upgrade` and `migrate` functions with access control
///   (`_require_auth`).
/// - Sets the current crate version  as `"binver"` metadata using
///   `soroban_sdk::contractmeta!`. Gets the crate version via the env variable
///   `CARGO_PKG_VERSION` which corresponds to the "version" attribute in
///   Cargo.toml. If no such attribute or if it is "0.0.0", skips this step.
/// - Throws a compile-time error if `UpgradeableMigratableInternal` is not
///   implemented.
///
/// # Example
/// ```ignore,rust
/// #[derive(UpgradeableMigratable)]
/// pub struct MyContract;
/// ```
///
/// **Warning:** This derive macro should only be used on contracts that have
/// previously used either `#[derive(Upgradeable)]` or
/// `#[derive(UpgradeableMigratable)]`. The migration function depends on an
/// internal flag set by calling `stellar_upgradeable::enable_migration()`.
pub fn derive_upgradeable_migratable(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let binver = set_binver_from_env();

    quote! {
        use stellar_upgradeable::UpgradeableMigratable as _;

        #binver

        type MigrationData = <#name as stellar_upgradeable::UpgradeableMigratableInternal>::MigrationData;

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::UpgradeableMigratable for #name {

            fn upgrade(
                e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>, operator: soroban_sdk::Address
            ) {
                Self::_require_auth(e, &operator);

                stellar_upgradeable::enable_migration(e);

                e.deployer().update_current_contract_wasm(new_wasm_hash);
            }

            fn migrate(e: &soroban_sdk::Env, migration_data: MigrationData, operator: soroban_sdk::Address) {
                Self::_require_auth(e, &operator);

                stellar_upgradeable::ensure_can_complete_migration(e);

                Self::_migrate(e, &migration_data);

                stellar_upgradeable::complete_migration(e);
            }
        }
    }
}

/// Sets the value of the environment variable `CARGO_PKG_VERSION` as `binver`
/// in the wasm binary metadata. This env variable corresponds to the attribute
/// "version" in Cargo.toml. If the attribute is missing or if it is "0.0.0",
/// the function does nothing.
fn set_binver_from_env() -> proc_macro2::TokenStream {
    // However when "version" is missing from Cargo.toml,
    // the following does not return error, but Ok("0.0.0")
    let mut tokens = quote! {};
    if let Ok(v) = std::env::var("CARGO_PKG_VERSION") {
        if v != "0.0.0" {
            tokens = quote! {
                soroban_sdk::contractmeta!(key = "binver", val = #v);
            };
        }
    }
    tokens
}
