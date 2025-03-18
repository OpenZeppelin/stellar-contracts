use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

pub fn derive_upgradeable(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;

    let version = env!("CARGO_PKG_VERSION");

    let with_migration = input.attrs.iter().find(|attr| attr.path().is_ident("migrateable"));
    let migrateable = match with_migration {
        Some(attr) => {
            if attr.meta.require_path_only().is_err() {
                panic!("migrateable attribute cannot have arguments")
            }
            derive_migrateable(name)
        }
        None => quote! {},
    };

    quote! {
        use stellar_upgradeable::Upgradeable as _;

        soroban_sdk::contractmeta!(key = "binver", val = #version);

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::Upgradeable for #name {
            fn upgrade(e: &soroban_sdk::Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
                stellar_upgradeable::start_migration(e);

                Self::_upgrade(e, &new_wasm_hash)
            }
        }

        #migrateable
    }
}

fn derive_migrateable(name: &Ident) -> proc_macro2::TokenStream {
    quote! {
        use stellar_upgradeable::Migrateable as _;

        type MigrationData = <#name as stellar_upgradeable::Migration>::MigrationData;
        type RollbackData = <#name as stellar_upgradeable::Migration>::RollbackData;

        #[soroban_sdk::contractimpl]
        impl stellar_upgradeable::Migrateable for #name {

            fn migrate(e: &soroban_sdk::Env, migration_data: MigrationData) {
                stellar_upgradeable::ensure_can_migrate(e);

                Self::_migrate(e, &migration_data);

                stellar_upgradeable::complete_migration(e);
            }

            fn rollback(e: &soroban_sdk::Env, rollback_data: RollbackData) {
                stellar_upgradeable::ensure_can_rollback(e);

                Self::_migrate(e, &rollback_data);

                stellar_upgradeable::complete_rollback(e);
            }
        }
    }
}
