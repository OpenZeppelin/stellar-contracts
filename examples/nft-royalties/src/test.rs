#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn test_default_royalty() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint a token
    let token_id = client.mint(&owner);

    // Check royalty info (should use default 10%)
    let (receiver, amount) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(receiver, owner);
    assert_eq!(amount, 100); // 10% of 1000
}

#[test]
fn test_token_specific_royalty() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let royalty_receiver = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint a token with specific royalty (5%)
    let token_id = client.mint_with_royalty(&owner, &royalty_receiver, &500);

    // Check royalty info
    let (receiver, amount) = client.get_royalty_info(&token_id, &2000);
    assert_eq!(receiver, royalty_receiver);
    assert_eq!(amount, 100); // 5% of 2000

    // Mint a regular token (should use default royalty)
    let regular_token_id = client.mint(&owner);

    // Check royalty info for regular token
    let (receiver, amount) = client.get_royalty_info(&regular_token_id, &2000);
    assert_eq!(receiver, owner);
    assert_eq!(amount, 200); // 10% of 2000
}

#[test]
fn test_zero_royalty() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let royalty_receiver = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint a token with zero royalty
    let token_id = client.mint_with_royalty(&owner, &royalty_receiver, &0);

    // Check royalty info
    let (receiver, amount) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(receiver, royalty_receiver);
    assert_eq!(amount, 0); // 0% royalty
}
