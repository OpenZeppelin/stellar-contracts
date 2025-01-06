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
fn test_initial_state() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert_eq!(paused(&e), false);
    });
}

#[test]
fn test_pause() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Test pause
        pause(&e, &caller);
        assert_eq!(paused(&e), true);

        //assert_eq!(
            //e.auths(),
            //[(
                //caller.clone(),
                //AuthorizedInvocation {
                    //function: AuthorizedFunction::Contract((
                        //address.clone(),
                        //symbol_short!("pause"),
                        //vec![&e, caller.clone().into_val(&e)]
                    //)),
                    //sub_invocations: [].to_vec()
                //}
            //)]
        //);

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
fn test_unpause() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let caller = Address::generate(&e);

    e.as_contract(&address, || {
        // Manually set storage
        e.storage().instance().set(&PAUSED, &true);

        // Test unpause
        unpause(&e, &caller);
        assert_eq!(paused(&e), false);
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
fn test_pause_when_paused() {
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
fn test_unpause_when_not_paused() {
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
fn test_when_not_paused() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Should not panic when contract is not paused
        when_not_paused(&e);
    });
}

#[test]
fn test_when_paused() {
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
