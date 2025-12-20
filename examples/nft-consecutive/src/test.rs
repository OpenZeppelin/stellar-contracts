extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let uri = String::from_str(e, "www.mytoken.com");
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (uri, name, symbol, owner));
    ExampleContractClient::new(e, &address)
}

#[test]
fn consecutive_transfer_override_works() {
    let e = Env::default();

    let owner = Address::generate(&e);

    let recipient = Address::generate(&e);

    let client = create_client(&e, &owner);

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
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);
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
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 99);
}

#[test]
fn consecutive_burn_override_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    assert_eq!(client.owner_of(&50), owner);
    client.burn(&owner, &50);
    assert_eq!(client.balance(&owner), 99);
    // Verify ownership is preserved for adjacent tokens
    assert_eq!(client.owner_of(&49), owner);
    assert_eq!(client.owner_of(&51), owner);
}

#[test]
fn consecutive_burn_from_override_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();
    client.batch_mint(&owner, &100);
    client.approve(&owner, &spender, &50, &1000);
    client.burn_from(&spender, &owner, &50);
    assert_eq!(client.balance(&owner), 99);
    // Verify ownership is preserved for adjacent tokens
    assert_eq!(client.owner_of(&49), owner);
    assert_eq!(client.owner_of(&51), owner);
}
