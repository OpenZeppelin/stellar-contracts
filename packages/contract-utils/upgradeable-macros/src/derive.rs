use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

/// Procedural macro implementation for `#[derive(Upgradeable)]`.
///
/// This function generates the implementation of the `Upgradeable` trait for a
/// given contract type, enabling the contract to be upgraded by replacing its
/// WASM bytecode.
///
/// If the contract struct is annotated with the `#[migratable]` attribute, this
/// macro will also generate an implementation of the `Migratable` trait, wiring
/// up the migration and rollback logic based on the `MigratableInternal` trait
/// provided by the user.
///
/// # Behavior
///
/// - Sets the current crate version (`CARGO_PKG_VERSION`) as `"binver"`
///   metadata using `contractmeta!`.
/// - Implements the `upgrade` function with access control (`_upgrade_auth`).
/// - Optionally implements the `migrate` and `rollback` functions for the
///   `Migratable` trait, if `#[migratable]` is present.
///
/// # Panics
///
/// Panics at compile time if `#[migratable]` is used with arguments.
///
/// # Example
/// ```ignore,rust
/// #[derive(Upgradeable)]
/// #[migratable]
/// pub struct MyContract;
/// ```
pub fn derive_upgradeable(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let version = env!("CARGO_PKG_VERSION");

    let with_migration = input.attrs.iter().find(|attr| attr.path().is_ident("migratable"));
    let migratable = match with_migration {
        Some(attr) => {
            if attr.meta.require_path_only().is_err() {
                panic!("migratable attribute cannot have arguments")
            }
            derive_migratable(name)
        }
        None => quote! {},
    };

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

        #migratable
    }
}

// Internal helper to derive the `Migratable` trait
fn derive_migratable(name: &Ident) -> proc_macro2::TokenStream {
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
