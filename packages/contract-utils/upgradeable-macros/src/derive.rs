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
/// - Implements the `upgrade` function with access control (`_upgrade_auth`).
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
            fn upgrade(e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>, operator: soroban_sdk::Address) {
                Self::_upgrade_auth(e, &operator);

                stellar_upgradeable::start_migration(e);

                e.deployer().update_current_contract_wasm(new_wasm_hash);
            }
        }
    }
}

/// Procedural macro implementation for `#[derive(Migratable)]`.
///
/// This function generates the implementation of the `Migratable` trait for a
/// given contract type, wiring up the migration and rollback logic based on the
/// `MigratableInternal` trait provided by the user.
///
/// **IMPORTANT**
///   It is highly recommended to use this derive macro as a combination with
///   `Upgradeable`: `#[derive(Upgradeable, Migratable)]`. Otherwise, you need
///   to ensure the upgradeability state transitions as defined in the crate
///   "stellar_upgradeable".
///
/// # Behavior
///
/// - Implements the `migrate` and `rollback` functions for the `Migratable`
///   trait.
/// - Throws a compile-time error if `MigratableInternal` is not implemented.
///
/// # Example
/// ```ignore,rust
/// #[derive(Upgradeable, Migratable)]
/// pub struct MyContract;
/// ```
pub fn derive_migratable(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    quote! {
        use stellar_upgradeable::Migratable as _;

        type MigrationData = <#name as stellar_upgradeable::MigratableInternal>::MigrationData;
        type RollbackData = <#name as stellar_upgradeable::MigratableInternal>::RollbackData;

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::Migratable for #name {

            fn migrate(e: &soroban_sdk::Env, migration_data: MigrationData) {
                stellar_upgradeable::ensure_can_migrate(e);

                Self::_migrate(e, &migration_data);

                stellar_upgradeable::complete_migration(e);
            }

            fn rollback(e: &soroban_sdk::Env, rollback_data: RollbackData) {
                stellar_upgradeable::ensure_can_rollback(e);

                Self::_rollback(e, &rollback_data);

                stellar_upgradeable::complete_rollback(e);
            }
        }
    }
}
