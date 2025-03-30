/// 1. Derives Upgradeable a) implements the interface; requires only the auth
///    to be defined b) sets wasm version by taking the version from Cargo.toml
///
/// 2. Optionally derives Migratable when migration and rollback are defined.
///
///
/// Example:
/// ```rust,ignore
/// #[derive(Upgradeable, Migratable)]
/// #[contract]
/// pub struct ExampleContract;
///
/// impl Upgrade for ExampleContract {
///     fn upgrade_auth(e: &Env) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
///     }
/// }
///
/// impl Migration for ExampleContract {
///     type MigrationData = Data;
///     type RollbackData = ();
///
///     fn _migrate(e: &Env, data: &Self::MigrationData) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
///     }
///
///     fn _rollback(e: &Env, _data: &Self::RollbackData) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
///         e.storage().instance().remove(&DATA_KEY);
///     }
/// }
/// ```
mod derive;

use derive::{derive_migratable, derive_upgradeable};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Upgradeable)]
pub fn upgradeable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_upgradeable(&input).into()
}

#[proc_macro_derive(Migratable)]
pub fn migratable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_migratable(&input).into()
}
