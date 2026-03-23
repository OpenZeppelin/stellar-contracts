extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (owner,));
    ExampleContractClient::new(e, &address)
}

#[test]
fn mint_and_delegate_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint tokens to user1
    client.mint(&user1, &1000);
    assert_eq!(client.balance(&user1), 1000);

    // Delegate user1's votes to user2
    client.delegate(&user1, &user2);
    assert_eq!(client.get_votes(&user2), 1000);
}

#[test]
fn burn_updates_delegate_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint tokens and delegate
    client.mint(&user1, &1000);
    client.delegate(&user1, &delegate);
    assert_eq!(client.get_votes(&delegate), 1000);

    // Burn reduces delegate's votes
    client.burn(&user1, &400);
    assert_eq!(client.balance(&user1), 600);
    assert_eq!(client.get_votes(&delegate), 600);
}

#[test]
fn burn_self_delegated_updates_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint and self-delegate
    client.mint(&user1, &1000);
    client.delegate(&user1, &user1);
    assert_eq!(client.get_votes(&user1), 1000);

    // Burn reduces own votes
    client.burn(&user1, &400);
    assert_eq!(client.balance(&user1), 600);
    assert_eq!(client.get_votes(&user1), 600);
}

#[test]
fn burn_from_updates_delegate_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let spender = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint tokens, delegate, and approve spender
    client.mint(&user1, &1000);
    client.delegate(&user1, &delegate);
    client.approve(&user1, &spender, &500, &1000);
    assert_eq!(client.get_votes(&delegate), 1000);

    // burn_from reduces delegate's votes
    client.burn_from(&spender, &user1, &300);
    assert_eq!(client.balance(&user1), 700);
    assert_eq!(client.get_votes(&delegate), 700);
}

#[test]
fn burn_all_tokens_zeroes_delegate_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let delegate = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint tokens and delegate
    client.mint(&user1, &1000);
    client.delegate(&user1, &delegate);
    assert_eq!(client.get_votes(&delegate), 1000);

    // Burn all tokens
    client.burn(&user1, &1000);
    assert_eq!(client.balance(&user1), 0);
    assert_eq!(client.get_votes(&delegate), 0);
}

#[test]
fn transfer_updates_delegate_votes() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let delegate1 = Address::generate(&e);
    let delegate2 = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    // Mint and delegate
    client.mint(&user1, &1000);
    client.mint(&user2, &500);
    client.delegate(&user1, &delegate1);
    client.delegate(&user2, &delegate2);

    // Transfer moves votes between delegates
    client.transfer(&user1, &user2, &300);
    assert_eq!(client.get_votes(&delegate1), 700);
    assert_eq!(client.get_votes(&delegate2), 800);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn burn_insufficient_balance_panics() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user1, &100);
    client.burn(&user1, &150);
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn burn_from_insufficient_allowance_panics() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let user1 = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e, &owner);

    e.mock_all_auths();

    client.mint(&user1, &1000);
    client.approve(&user1, &spender, &200, &1000);
    client.burn_from(&spender, &user1, &300);
}
