use soroban_sdk::{panic_with_error, symbol_short, Env, Symbol};

use crate::upgradeable::UpgradeableError;

pub const MIGRATING: Symbol = symbol_short!("MIGRATING");

/// Sets the `MIGRATING` state to `true`, enabling migration process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn enable_migration(e: &Env) {
    e.storage().instance().set(&MIGRATING, &true);
}

/// Returns `true` if completing migration is allowed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn can_complete_migration(e: &Env) -> bool {
    e.storage().instance().get::<_, bool>(&MIGRATING).unwrap_or(false)
}

/// Sets the `MIGRATING` state to `false`, completing the migration process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn complete_migration(e: &Env) {
    e.storage().instance().set(&MIGRATING, &false);
}

/// Ensures that completing migration is allowed, otherwise panics.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
///
/// # Errors
///
/// * [`UpgradeableError::MigrationNotAllowed`] - If `MIGRATING` is `false`.
pub fn ensure_can_complete_migration(e: &Env) {
    if !can_complete_migration(e) {
        panic_with_error!(e, UpgradeableError::MigrationNotAllowed)
    }
}
