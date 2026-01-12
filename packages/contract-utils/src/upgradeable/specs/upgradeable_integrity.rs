use cvlr::{cvlr_assert, cvlr_satisfy, nondet, cvlr_assume, clog};
use cvlr_soroban::{nondet_address, nondet_bytes_n};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::upgradeable::{enable_migration, can_complete_migration, complete_migration};
use crate::upgradeable::specs::upgradeable_migratable_contract::UpgradeableMigratableContract;
use crate::upgradeable::UpgradeableMigratable;

#[rule]
// after enable_migration can_complete_migration returns true
// status: spurious violation 
// storage just does not behave as expected - quite silly.
pub fn enable_migration_integrity(e: Env) {
    let can_complete_migration_pre = can_complete_migration(&e);
    clog!(can_complete_migration_pre);
    enable_migration(&e);
    let can_complete_migration_post = can_complete_migration(&e);
    clog!(can_complete_migration_post);
    cvlr_assert!(can_complete_migration_post);
}

#[rule]
// after complete_migration can_complete_migration returns false
// status: spurious violation 
// same as above.
pub fn complete_migration_integrity(e: Env) {
    let can_complete_migration_pre = can_complete_migration(&e);
    clog!(can_complete_migration_pre);
    complete_migration(&e);
    let can_complete_migration_post = can_complete_migration(&e);
    clog!(can_complete_migration_post);
    cvlr_assert!(!can_complete_migration_post);
}

// integrity rules for the contract upgrade and migrate functions

#[rule]
// after upgrade can_complete_migration is true
// status: spurious violation - as above
pub fn upgrade_integrity_1(e: Env) {
    let wasm_hash: soroban_sdk::BytesN<32> = nondet_bytes_n();
    clog!(cvlr_soroban::BN(&wasm_hash));
    let operator = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    UpgradeableMigratableContract::upgrade(&e, wasm_hash, operator);
    let can_complete_migration_post = can_complete_migration(&e);
    clog!(can_complete_migration_post);
    cvlr_assert!(can_complete_migration_post);
}

// after upgrade the current contract wasm is the given hash
// we can't write it because there is no way to access the wasm

#[rule]
// after migrate can_complete_migration is false
// status: spurious violation - as above
pub fn migrate_integrity_1(e: Env) {
    let migrate_data: u32 = nondet();
    let operator = nondet_address();
    UpgradeableMigratableContract::migrate(&e, migrate_data, operator);
    let can_complete_migration_post = can_complete_migration(&e);
    clog!(can_complete_migration_post);
    cvlr_assert!(!can_complete_migration_post);
}

// migrate does not change contract wasm
// we can't write it because there is no way to access the wasm