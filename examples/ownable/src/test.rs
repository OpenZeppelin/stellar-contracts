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

    assert_eq!(client.increment(&owner), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")]
fn non_owner_cannot_increment() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.increment(&user);
}
