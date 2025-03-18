use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env, Symbol};

use crate::upgradeable::UpgradeableError;

pub const UPGRADE_KEY: Symbol = symbol_short!("upgrade");

#[contracttype]
pub enum UpgradeState {
    Initial,
    Migrated,
    RolledBack,
}

pub fn start_migration(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::Initial);
}

// only after initial
pub fn can_migrate(e: &Env) -> bool {
    matches!(get_upgrade_state(e), UpgradeState::Initial)
}

// only after migratied
pub fn can_rollback(e: &Env) -> bool {
    matches!(get_upgrade_state(e), UpgradeState::Migrated)
}

pub fn complete_migration(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::Migrated);
}

pub fn complete_rollback(e: &Env) {
    e.storage().instance().set(&UPGRADE_KEY, &UpgradeState::RolledBack);
}

pub fn ensure_can_migrate(e: &Env) {
    if !can_migrate(e) {
        panic_with_error!(e, UpgradeableError::MigrationNotAllowed)
    }
}

pub fn ensure_can_rollback(e: &Env) {
    if !can_rollback(e) {
        panic_with_error!(e, UpgradeableError::RollbackNotAllowed)
    }
}

pub(crate) fn get_upgrade_state(e: &Env) -> UpgradeState {
    match e.storage().instance().get::<_, UpgradeState>(&UPGRADE_KEY) {
        Some(state) => state,
        None => UpgradeState::Initial,
    }
}
