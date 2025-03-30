use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env, Symbol};

use crate::upgradeable::UpgradeableError;

pub const UPGRADE_KEY: Symbol = symbol_short!("UPGRADE");

/// Represents the current upgrade state of the contract. Used to determine if
/// migration or rollback operations are allowed.
#[contracttype]
pub enum UpgradeState {
    Initial,
    Migrated,
    RolledBack,
}

/// Sets the upgrade state to `Initial`, indicating the beginning of a migration
/// process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn start_migration(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::Initial);
}

/// Returns `true` if migration is allowed, which is only when the state is
/// `Initial`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn can_migrate(e: &Env) -> bool {
    matches!(get_upgrade_state(e), UpgradeState::Initial)
}

/// Returns `true` if rollback is allowed, which is only when the state is
/// `Migrated`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn can_rollback(e: &Env) -> bool {
    matches!(get_upgrade_state(e), UpgradeState::Migrated)
}

/// Sets the upgrade state to `Migrated`, completing the migration process.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn complete_migration(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::Migrated);
}

/// Sets the upgrade state to `RolledBack`, indicating a completed rollback.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn complete_rollback(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::RolledBack);
}

/// Ensures that migration is allowed, otherwise panics.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
///
/// # Errors
///
/// * [`UpgradeableError::MigrationNotAllowed`] - If state is not `Initial`.
pub fn ensure_can_migrate(e: &Env) {
    if !can_migrate(e) {
        panic_with_error!(e, UpgradeableError::MigrationNotAllowed)
    }
}

/// Ensures that rollback is allowed, otherwise panics.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
///
/// # Errors
///
/// * [`UpgradeableError::RollbackNotAllowed`] - If state is not `Migrated`.
pub fn ensure_can_rollback(e: &Env) {
    if !can_rollback(e) {
        panic_with_error!(e, UpgradeableError::RollbackNotAllowed)
    }
}

/// Retrieves the current upgrade state from instance storage.
///
/// If no state has been set, defaults to [`UpgradeState::Initial`].
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub(crate) fn get_upgrade_state(e: &Env) -> UpgradeState {
    match e.storage().instance().get::<_, UpgradeState>(&UPGRADE_KEY) {
        Some(state) => state,
        None => UpgradeState::Initial,
    }
}
