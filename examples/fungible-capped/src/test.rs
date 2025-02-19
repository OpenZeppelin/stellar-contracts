#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, cap: &i128) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (cap,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn mint_under_cap() {
    let e = Env::default();
    let cap = 1000;
    let client = create_client(&e, &cap);
    let user = Address::generate(&e);

    client.mint(&user, &500);

    assert_eq!(client.balance(&user), 500);
    assert_eq!(client.total_supply(), 500);
}

#[test]
fn mint_exact_cap() {
    let e = Env::default();
    let cap = 1000;
    let client = create_client(&e, &cap);
    let user = Address::generate(&e);

    client.mint(&user, &1000);

    assert_eq!(client.balance(&user), 1000);
    assert_eq!(client.total_supply(), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")]
fn mint_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let client = create_client(&e, &cap);
    let user = Address::generate(&e);

    // Attempt to mint 1001 tokens (would exceed cap)
    client.mint(&user, &1001); // This should panic
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")]
fn mint_multiple_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let client = create_client(&e, &cap);
    let user = Address::generate(&e);

    // Mint 600 tokens first
    client.mint(&user, &600);

    assert_eq!(client.balance(&user), 600);
    assert_eq!(client.total_supply(), 600);

    // Attempt to mint 500 more tokens (would exceed cap)
    client.mint(&user, &500); // This should panic
}
