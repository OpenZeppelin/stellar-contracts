#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, ());
    ExampleContractClient::new(e, &address)
}

#[test]
fn consecutive_transfer_override_works() {
    let e = Env::default();

    let owner = Address::generate(&e);

    let recipient = Address::generate(&e);

    let client = create_client(&e);

    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    client.transfer(&owner, &recipient, &10);
    assert_eq!(client.balance(&owner), 99);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.owner_of(&10), recipient);
}

#[test]
fn consecutive_batch_mint_works() {
    let e = Env::default();
    let client = create_client(&e);
    let owner = Address::generate(&e);
    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 99);
    client.batch_mint(&owner, &100);
    assert_eq!(client.owner_of(&101), owner);
}

#[test]
fn consecutive_burn_works() {
    let e = Env::default();
    let client = create_client(&e);
    let owner = Address::generate(&e);
    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 99);
}
