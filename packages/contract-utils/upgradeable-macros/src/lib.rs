/// 1. Derives Upgradeable a) implements the interface; requires only the auth
///    to be defined b) sets wasm version by taking the version from Cargo.toml
///
/// 2. Derives UpgradeableMigratable when both, an upgrade and a migration are
///    needed a) implements the interface; requires the auth and the migration
///    logic to be defined b) sets wasm version by taking the version from
///    Cargo.toml
///
///
/// Example for upgrade only:
/// ```rust,ignore
/// #[derive(Upgradeable)]
/// #[contract]
/// pub struct ExampleContract;
///
/// impl UpgradeableInternal for ExampleContract {
///     fn _require_auth(e: &Env) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
///     }
/// ```
///
/// Example for upgrade and migration:
/// ```rust,ignore
/// #[derive(UpgradeableMigratable)]
/// #[contract]
/// pub struct ExampleContract;
///
/// impl UpgradeableMigratableInternal for ExampleContract {
///     type MigrationData = Data;
///
///     fn _require_auth(e: &Env) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
///     }
///
///     fn _migrate(e: &Env, data: &Self::MigrationData) {
///         e.storage().instance().get::<_, Address>(&OWNER).unwrap().require_auth();
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

#[proc_macro_derive(UpgradeableMigratable)]
pub fn migratable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_migratable(&input).into()
}
