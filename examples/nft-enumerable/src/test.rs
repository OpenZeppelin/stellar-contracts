#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn enumerable_transfer_override_works() {
    let e = Env::default();

    let owner = Address::generate(&e);

    let recipient = Address::generate(&e);

    let client = create_client(&e);

    e.mock_all_auths();
    client.mint(&owner);
    client.transfer(&owner, &recipient, &0);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.get_owner_token_id(&recipient, &0), 0);
}

#[test]
fn enumerable_burn_works() {
    let e = Env::default();
    let client = create_client(&e);
    let owner = Address::generate(&e);
    e.mock_all_auths();
    client.mint(&owner);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 0);
    client.mint(&owner);
    assert_eq!(client.balance(&owner), 1);
    assert_eq!(client.get_owner_token_id(&owner, &0), 1);
}
