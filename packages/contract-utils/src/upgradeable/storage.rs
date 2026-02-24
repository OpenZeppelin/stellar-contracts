use soroban_sdk::{contracttype, panic_with_error, BytesN, Env};

use crate::upgradeable::UpgradeableError;

#[contracttype]
pub enum UpgradeableStorageKey {
    Migrating,
}

/// Enables the migration flag and updates the contract WASM.
///
/// Call this inside a [`Upgradeable::upgrade`] implementation after all
/// authorization checks have passed. The migration flag signals to the next
/// contract version that a migration could be expected.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `new_wasm_hash` - A 32-byte hash identifying the new WASM blob, uploaded
///   to the ledger.
pub fn upgrade(e: &Env, new_wasm_hash: &BytesN<32>) {
    enable_migration(e);
    e.deployer().update_current_contract_wasm(new_wasm_hash.clone());
}

/// Runs the migration closure and completes the migration.
///
/// Call this inside a `migrate` function after all authorization checks
/// have passed. It can be called ONLY after the migration flag was set to
/// `true` (as in [`upgrade`]).
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `f` - A closure containing the migration logic to execute.
///
/// # Errors
///
/// * [`UpgradeableError::MigrationNotAllowed`] - If the migration state is not
///   set.
pub fn run_migration(e: &Env, f: impl FnOnce()) {
    ensure_can_complete_migration(e);
    f();
    complete_migration(e);
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
