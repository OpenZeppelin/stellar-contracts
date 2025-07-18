#![cfg(test)]

use soroban_sdk::{contract, Env};

use crate::upgradeable::storage::{
    can_complete_migration, complete_migration, enable_migration, ensure_can_complete_migration,
};

#[contract]
struct MockContract;

#[test]
fn upgrade_flow_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert!(!can_complete_migration(&e));

        enable_migration(&e);
        assert!(can_complete_migration(&e));

        complete_migration(&e);
        assert!(!can_complete_migration(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1100)")]
fn upgrade_ensure_can_complete_migration_panics_if_not_migrating() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        complete_migration(&e);
        ensure_can_complete_migration(&e);
    });
}
