#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

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
#[should_panic(expected = "Unauthorized function call for address")]
fn errors_pause_unauthorized() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "pause",
            args: (&user,).into_val(&e),
            sub_invokes: &[],
        },
    }]);
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
#[should_panic(expected = "Unauthorized function call for address")]
fn errors_unpause_unauthorized() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_auths(&[MockAuth {
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "pause",
            args: (&owner,).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    client.pause(&owner);

    e.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "unpause",
            args: (&owner,).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.unpause(&owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn errors_increment_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.pause(&owner);
    client.increment();
}

#[test]
#[should_panic(expected = "Error(Contract, #1001)")]
fn errors_emergency_reset_when_not_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.emergency_reset();
}
