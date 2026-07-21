extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    Address, Env, Event,
};

use crate::rwa::compliance::modules::supply_limit::{
    storage::{
        get_supply_count, get_supply_limit, is_preset_completed, mark_preset_completed, on_created,
        on_destroyed, preset_supply_count, set_supply_limit,
    },
    PresetCompleted,
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
fn on_created_allows_minting_up_to_the_limit() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);
        on_created(&e, &to, 70, &token);
        // The remaining headroom can still be minted exactly.
        on_created(&e, &to, 30, &token);

        assert_eq!(get_supply_count(&e, &token), 100);
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
#[should_panic(expected = "Error(Contract, #394)")]
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
#[should_panic(expected = "Error(Contract, #392)")]
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
#[should_panic(expected = "Error(Contract, #390)")]
fn set_supply_limit_panics_on_negative_value() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn on_created_panics_on_negative_amount() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        on_created(&e, &to, -1, &token);
    });
}

#[test]
fn preset_supply_count_seeds_counter_for_migration() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let holder = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 1_000);
        // The token migrates in with 600 already circulating.
        preset_supply_count(&e, &token, 600);
        assert_eq!(get_supply_count(&e, &token), 600);
        assert!(!is_preset_completed(&e, &token));

        // Burning pre-existing tokens works instead of underflowing, and
        // the cap is measured against the real total supply.
        on_destroyed(&e, &holder, 100, &token);
        on_created(&e, &holder, 500, &token);
        assert_eq!(get_supply_count(&e, &token), 1_000);
    });
}

#[test]
fn preset_supply_count_overwrites_until_finalized() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_supply_count(&e, &token, 300);
        preset_supply_count(&e, &token, 450);
        assert_eq!(get_supply_count(&e, &token), 450);
    });
}

#[test]
fn preset_supply_count_may_exceed_limit() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let holder = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_supply_limit(&e, &token, 100);
        // A migrated token can already sit above the intended cap; the
        // counter must still reflect reality.
        preset_supply_count(&e, &token, 150);
        assert_eq!(get_supply_count(&e, &token), 150);

        // Burns bring the tracked supply back under the limit, after which
        // minting the remaining headroom succeeds.
        on_destroyed(&e, &holder, 60, &token);
        on_created(&e, &holder, 10, &token);
        assert_eq!(get_supply_count(&e, &token), 100);
    });
}

#[test]
fn mark_preset_completed_blocks_further_presets() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        assert!(!is_preset_completed(&e, &token));
        mark_preset_completed(&e, &token);
        assert!(is_preset_completed(&e, &token));

        let after_mark = e.events().all().events().len();
        // Repeated marking is a write but emits another event; no panic.
        mark_preset_completed(&e, &token);
        let events = e.events().all();
        assert_eq!(events.events().len(), after_mark + 1);
        assert_eq!(
            events.events().get(after_mark).unwrap(),
            &PresetCompleted { token: token.clone() }.to_xdr(&e, &module_id)
        );
        assert!(is_preset_completed(&e, &token));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #395)")]
fn preset_supply_count_panics_after_preset_completed() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        mark_preset_completed(&e, &token);
        preset_supply_count(&e, &token, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #390)")]
fn preset_supply_count_panics_on_negative_supply() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);

    e.as_contract(&module_id, || {
        preset_supply_count(&e, &token, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #391)")]
fn on_created_panics_on_supply_count_overflow() {
    let e = Env::default();
    let module_id = e.register(TestSupplyLimitContract, ());
    let token = Address::generate(&e);
    let to = Address::generate(&e);

    e.as_contract(&module_id, || {
        // The preset can seed the counter at the numeric ceiling, so the
        // addition in `on_created` must trip before the cap check does.
        set_supply_limit(&e, &token, i128::MAX);
        preset_supply_count(&e, &token, i128::MAX);
        on_created(&e, &to, 1, &token);
    });
}
