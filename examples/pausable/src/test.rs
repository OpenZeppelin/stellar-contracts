#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn initial_state() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    assert!(!client.paused());
    assert_eq!(client.increment(), 1);
}

#[test]
fn pause_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&owner);

    assert!(client.paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn errors_pause_unauthorized() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&user);
}

#[test]
fn unpause_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&owner);
    client.unpause(&owner);

    assert!(!client.paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn errors_unpause_unauthorized() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&owner);
    client.unpause(&user);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn errors_increment_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&owner);
    client.increment();
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn errors_emergency_reset_when_not_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.emergency_reset();
}
