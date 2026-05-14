extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events as _},
    Address, Env,
};

use crate::rwa::compliance::modules::supply_limit::storage::{
    can_create, can_transfer, get_supply_count, get_supply_limit, on_created, on_destroyed,
    set_supply_limit,
};

#[contract]
struct TestSupplyLimitContract;

#[test]
fn set_supply_limit_persists_value() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 1_000);
        assert_eq!(get_supply_limit(&e, &token), 1_000);
        assert_eq!(get_supply_count(&e, &token), 0);
    });
}

#[test]
fn can_create_allows_under_limit_and_rejects_at_overflow() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);

        assert!(can_create(&e, &to, 100, &token));
        assert!(can_create(&e, &to, 0, &token));
        assert!(!can_create(&e, &to, 101, &token));
    });
}

#[test]
fn can_create_accounts_for_running_supply() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);
        on_created(&e, &to, 70, &token);

        assert!(can_create(&e, &to, 30, &token));
        assert!(!can_create(&e, &to, 31, &token));
    });
}

#[test]
fn can_transfer_is_always_true() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert!(can_transfer(&e, &from, &to, 9_999, &token));
    });
}

#[test]
fn on_created_increments_counter_and_emits_event() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);
        on_created(&e, &to, 40, &token);
        on_created(&e, &to, 30, &token);

        assert_eq!(get_supply_count(&e, &token), 70);
        // SupplyLimitSet + two SupplyCountUpdated emissions.
        assert_eq!(e.events().all().events().len(), 3);
    });
}

#[test]
fn on_destroyed_decrements_counter() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);
        on_created(&e, &from, 80, &token);
        on_destroyed(&e, &from, 30, &token);

        assert_eq!(get_supply_count(&e, &token), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn on_created_panics_when_exceeding_limit() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 50);
        on_created(&e, &to, 51, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #393)")]
fn on_destroyed_panics_on_underflow() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let from = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_destroyed(&e, &from, 1, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #391)")]
fn set_supply_limit_panics_on_negative_value() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #391)")]
fn can_create_panics_on_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        can_create(&e, &to, -1, &token);
    });
}
