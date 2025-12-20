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
fn enumerable_transfer_override_works() {
    let e = Env::default();

    let owner = Address::generate(&e);

    let recipient = Address::generate(&e);

    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.mint(&owner);
    client.transfer(&owner, &recipient, &0);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.get_owner_token_id(&recipient, &0), 0);
}

#[test]
fn enumerable_transfer_from_override_works() {
    let e = Env::default();

    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    let client = create_client(&e, &owner);

    e.mock_all_auths();
    client.mint(&owner);
    client.approve(&owner, &spender, &0, &1000);
    client.transfer_from(&spender, &owner, &recipient, &0);
    assert_eq!(client.balance(&owner), 0);
    assert_eq!(client.balance(&recipient), 1);
    assert_eq!(client.get_owner_token_id(&recipient, &0), 0);
}

#[test]
fn enumerable_burn_override_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();
    client.mint(&owner);
    client.burn(&owner, &0);
    assert_eq!(client.balance(&owner), 0);
    client.mint(&owner);
    assert_eq!(client.balance(&owner), 1);
    assert_eq!(client.get_owner_token_id(&owner, &0), 1);
}

#[test]
fn enumerable_burn_from_override_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e, &owner);
    e.mock_all_auths();
    client.mint(&owner);
    client.approve(&owner, &spender, &0, &1000);
    client.burn_from(&spender, &owner, &0);
    assert_eq!(client.balance(&owner), 0);
    client.mint(&owner);
    assert_eq!(client.balance(&owner), 1);
    assert_eq!(client.get_owner_token_id(&owner, &0), 1);
}
