use cvlr::{cvlr_assert, cvlr_satisfy, nondet, cvlr_assume, clog};
use cvlr_soroban::{nondet_address, nondet_bytes_n, is_auth};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::upgradeable::{enable_migration, can_complete_migration, complete_migration};
use crate::upgradeable::specs::upgradeable_migratable_contract::UpgradeableMigratableContract;
use crate::upgradeable::UpgradeableMigratable;

#[rule]
// upgrade panics if not auth by owner
// status: verified
pub fn upgrade_panics_if_not_auth_by_owner(e: Env) {
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let owner = UpgradeableMigratableContract::get_owner(&e);
    clog!(cvlr_soroban::Addr(&owner));
    let wasm_hash: soroban_sdk::BytesN<32> = nondet_bytes_n();
    clog!(cvlr_soroban::BN(&wasm_hash));
    cvlr_assume!(!is_auth(operator.clone()));
    UpgradeableMigratableContract::upgrade(&e, wasm_hash, operator);
    cvlr_assert!(false);
}

#[rule]
// upgrade panics if operator != owner
// status: verified
pub fn upgrade_panics_if_operator_not_owner(e: Env) {
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let owner = UpgradeableMigratableContract::get_owner(&e);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(operator != owner);
    let wasm_hash: soroban_sdk::BytesN<32> = nondet_bytes_n();
    clog!(cvlr_soroban::BN(&wasm_hash));
    UpgradeableMigratableContract::upgrade(&e, wasm_hash, operator);
    cvlr_assert!(false);
}

#[rule]
// migrate panics if not auth by owner
// status: verified
pub fn migrate_panics_if_not_auth_by_owner(e: Env) {
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let owner = UpgradeableMigratableContract::get_owner(&e);
    clog!(cvlr_soroban::Addr(&owner));
    cvlr_assume!(!is_auth(operator.clone()));
    let migrate_data: u32 = nondet();
    clog!(migrate_data);
    UpgradeableMigratableContract::migrate(&e, migrate_data, operator);
    cvlr_assert!(false);
}

#[rule]
// migrate panics if migration has completed
// status: verified
pub fn migrate_panics_if_migration_has_completed(e: Env) {
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let owner = UpgradeableMigratableContract::get_owner(&e);
    clog!(cvlr_soroban::Addr(&owner));
    let migrate_data: u32 = nondet();
    clog!(migrate_data);
    let can_complete_migration = can_complete_migration(&e);
    clog!(can_complete_migration);
    cvlr_assume!(!can_complete_migration);
    UpgradeableMigratableContract::migrate(&e, migrate_data, operator);
    cvlr_assert!(false);
}