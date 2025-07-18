#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Events, vec, Env, IntoVal, Symbol};

use crate::pausable::storage::{
    pause, paused, unpause, when_not_paused, when_paused, PausableStorageKey,
};

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
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Test pause
        pause(&e);
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
                    ().into_val(&e)
                )
            ]
        );
    });
}

#[test]
fn unpause_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Manually set storage
        e.storage().instance().set(&PausableStorageKey::Paused, &true);

        // Test unpause
        unpause(&e);
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
                    ().into_val(&e)
                )
            ]
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn errors_pause_when_paused() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Manually set storage
        e.storage().instance().set(&PausableStorageKey::Paused, &true);
        // Should panic when trying to pause again
        pause(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1001)")]
fn errors_unpause_when_not_paused() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Should panic when trying to unpause while not paused
        unpause(&e);
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
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        pause(&e);
        // Should not panic when contract is paused
        when_paused(&e);
    });
}
