use cvlr::{cvlr_assert,cvlr_satisfy, nondet};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n};
use cvlr_soroban_derive::rule;

use soroban_sdk::{Env};

use crate::upgradeable::{specs::{upgradeable_migratable_contract::UpgradeableMigratableContract}, *};


// contract does not make sense because there are no default implementations for the upgradeable trait functions


#[rule]
pub fn enable_migration_sanity(e: Env) {
    enable_migration(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_complete_migration_sanity(e: Env) {
    can_complete_migration(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn complete_migration_sanity(e: Env) {
    complete_migration(&e);
    cvlr_satisfy!(true);
}


#[rule]
pub fn ensure_can_complete_migration_sanity(e: Env) {
    ensure_can_complete_migration(&e);
    cvlr_satisfy!(true);
}


// NOTE: should only run once relevant methods are implemented in the contract
#[rule]
pub fn upgradeable_migratable_contract_upgrade_sanity(e: Env) {
    let wasm_hash: soroban_sdk::BytesN<32> = nondet_bytes_n();
    let operator = nondet_address();
    UpgradeableMigratableContract::upgrade(&e, wasm_hash, operator);
    cvlr_satisfy!(true);
}

// NOTE: should only run once relevant methods are implemented in the contract
#[rule]
pub fn upgradeable_migratable_contract_migrate_sanity(e: Env) {
    let migrate_data: u32 = nondet();
    let operator = nondet_address();
    UpgradeableMigratableContract::migrate(&e, migrate_data, operator);
    cvlr_satisfy!(true);
}