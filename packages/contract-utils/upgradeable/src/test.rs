#![cfg(test)]

use soroban_sdk::{contract, Env};

use crate::storage::{
    can_migrate, can_rollback, complete_migration, complete_rollback, ensure_can_migrate,
    ensure_can_rollback, start_migration,
};

#[contract]
struct MockContract;

#[test]
fn upgrade_flow_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert!(can_migrate(&e));
        assert!(!can_rollback(&e));

        start_migration(&e);
        assert!(can_migrate(&e));
        assert!(!can_rollback(&e));

        complete_migration(&e);
        assert!(!can_migrate(&e));
        assert!(can_rollback(&e));

        complete_rollback(&e);
        assert!(!can_migrate(&e));
        assert!(!can_rollback(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")]
fn upgrade_ensure_can_migrate_panics_if_not_initial() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        complete_migration(&e);
        ensure_can_migrate(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn upgrade_ensure_can_rollback_panics_if_not_migrated() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        complete_rollback(&e);
        ensure_can_rollback(&e);
    });
}
