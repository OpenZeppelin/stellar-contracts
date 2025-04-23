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
/// - Sets the current crate version (`CARGO_PKG_VERSION`) as `"binver"`
///   metadata using `contractmeta!`.
/// - Implements the `upgrade` function with access control (`_require_auth`).
/// - Throws a compile-time error if `UpgradeableInternal` is not implemented.
///
/// # Example
/// ```ignore,rust
/// #[derive(Upgradeable)]
/// pub struct MyContract;
/// ```
pub fn derive_upgradeable(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let version = env!("CARGO_PKG_VERSION");

    quote! {
        use stellar_upgradeable::Upgradeable as _;

        soroban_sdk::contractmeta!(key = "binver", val = #version);

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::Upgradeable for #name {
            fn upgrade(
                e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>, operator: soroban_sdk::Address
            ) {
                Self::_require_auth(e, &operator);

                e.deployer().update_current_contract_wasm(new_wasm_hash);
            }
        }
    }
}

/// Procedural macro implementation for `#[derive(UpgradeableMigratable)]`.
///
/// This function generates the implementation of the `UpgradeableMigratable`
/// trait for a given contract type, wiring up the migration and rollback logic
/// based on the `UpgradeableMigratableInternal` trait provided by the user.
///
/// **IMPORTANT**
///   It is highly recommended to use this derive macro as a combination with
///   `Upgradeable`: `#[derive(UpgradeableMigratable)]`. Otherwise, you need
///   to ensure the upgradeability state transitions as defined in the crate
///   `stellar_upgradeable`.
///
/// # Behavior
///
/// - Implements the `migrate` and `rollback` functions for the
///   `UpgradeableMigratable` trait.
/// - Throws a compile-time error if `UpgradeableMigratableInternal` is not
///   implemented.
///
/// # Example
/// ```ignore,rust
/// #[derive(UpgradeableMigratable)]
/// pub struct MyContract;
/// ```
pub fn derive_migratable(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let version = env!("CARGO_PKG_VERSION");

    quote! {
        use stellar_upgradeable::UpgradeableMigratable as _;

        soroban_sdk::contractmeta!(key = "binver", val = #version);

        type MigrationData = <#name as stellar_upgradeable::UpgradeableMigratableInternal>::MigrationData;

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::UpgradeableMigratable for #name {

            fn upgrade(
                e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>, operator: soroban_sdk::Address
            ) {
                Self::_require_auth(e, &operator);

                stellar_upgradeable::start_migration(e);

                e.deployer().update_current_contract_wasm(new_wasm_hash);
            }

            fn migrate(e: &soroban_sdk::Env, migration_data: MigrationData, operator: soroban_sdk::Address) {
                Self::_require_auth(e, &operator);

                stellar_upgradeable::ensure_can_migrate(e);

                Self::_migrate(e, &migration_data);

                stellar_upgradeable::complete_migration(e);
            }
        }
    }
}
