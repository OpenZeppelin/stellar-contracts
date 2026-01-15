use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::upgradeable::UpgradeableError;

#[contracttype]
pub enum UpgradeableStorageKey {
    Migrating,
}

/// Sets the migrating state to `true`, enabling migration process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn enable_migration(e: &Env) {
    e.storage().instance().set(&UpgradeableStorageKey::Migrating, &true);
}

/// Returns `true` if completing migration is allowed.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn can_complete_migration(e: &Env) -> bool {
    e.storage().instance().get::<_, bool>(&UpgradeableStorageKey::Migrating).unwrap_or(false)
}

/// Sets the migrating state to `false`, completing the migration process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn complete_migration(e: &Env) {
    e.storage().instance().set(&UpgradeableStorageKey::Migrating, &false);
}

/// Ensures that completing migration is allowed, otherwise panics.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
///
/// # Errors
///
/// * [`UpgradeableError::MigrationNotAllowed`] - If the migrating state is
///   `false`.
pub fn ensure_can_complete_migration(e: &Env) {
    if !can_complete_migration(e) {
        panic_with_error!(e, UpgradeableError::MigrationNotAllowed)
    }
}
