#![cfg(test)]
extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    vec, Address, Env, IntoVal, Symbol,
};

use crate::storage::{pause, paused, unpause, when_not_paused, when_paused, PAUSED};

#[contract]
struct MockContract;

#[test]
fn initial_state() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert!(!paused(&e));
    });
}

#[test]
fn pause_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Test pause
        pause(&e, &caller);
        assert!(paused(&e));

        let events = e.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events,
            vec![
                &e,
                (
                    address.clone(),
                    vec![&e, Symbol::new(&e, "paused").into_val(&e)],
                    caller.into_val(&e)
                )
            ]
        );
    });
}

#[test]
fn unpause_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Manually set storage
        e.storage().instance().set(&PAUSED, &true);

        // Test unpause
        unpause(&e, &caller);
        assert!(!paused(&e));
        let events = e.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events,
            vec![
                &e,
                (
                    address.clone(),
                    vec![&e, Symbol::new(&e, "unpaused").into_val(&e)],
                    caller.into_val(&e)
                )
            ]
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn errors_pause_when_paused() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Manually set storage
        e.storage().instance().set(&PAUSED, &true);
        // Should panic when trying to pause again
        pause(&e, &caller);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn errors_unpause_when_not_paused() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Should panic when trying to unpause while not paused
        unpause(&e, &caller);
    });
}

#[test]
fn when_not_paused_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Should not panic when contract is not paused
        when_not_paused(&e);
    });
}

#[test]
fn when_paused_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        pause(&e, &caller);
        // Should not panic when contract is paused
        when_paused(&e);
    });
}
