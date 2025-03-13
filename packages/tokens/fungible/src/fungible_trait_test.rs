use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, IntoVal, String};

use crate::{fungible::FungibleToken, mintable::mint, storage};

#[contract]
pub struct TestToken;

#[contractimpl]
impl FungibleToken for TestToken {
    fn total_supply(e: &Env) -> i128 {
        storage::total_supply(e)
    }

    fn balance(e: &Env, owner: Address) -> i128 {
        storage::balance(e, &owner)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        storage::allowance(e, &owner, &spender)
    }

    fn name(e: &Env) -> String {
        "Test Token".into_val(e)
    }

    fn symbol(e: &Env) -> String {
        "TEST".into_val(e)
    }

    fn decimals(_e: &Env) -> u32 {
        7
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        // Skip authorization check in tests
        storage::transfer(e, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        // Skip authorization check in tests
        storage::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        // Skip authorization check in tests
        storage::transfer_from(e, &spender, &from, &to, amount);
    }
}

#[test]
fn test_fungible_token_trait() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register(TestToken, ());
    let client = TestTokenClient::new(&e, &contract_id);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Test implementation of the trait

    // Mint some tokens for testing (using the internal functions)
    e.as_contract(&contract_id, || {
        mint(&e, &user1, 1000);
    });

    // Test the trait functions
    assert_eq!(client.total_supply(), 1000);
    assert_eq!(client.balance(&user1), 1000);
    assert_eq!(client.balance(&user2), 0);
    assert_eq!(client.name(), "Test Token".into_val(&e));
    assert_eq!(client.symbol(), "TEST".into_val(&e));
    assert_eq!(client.decimals(), 7);

    // Test transfer function
    client.transfer(&user1, &user2, &300);
    assert_eq!(client.balance(&user1), 700);
    assert_eq!(client.balance(&user2), 300);

    // Test approve and allowance functions
    let expiration = e.ledger().sequence() + 1000;
    client.approve(&user1, &user2, &200, &expiration);
    assert_eq!(client.allowance(&user1, &user2), 200);

    // Test transfer_from function
    client.transfer_from(&user2, &user1, &user2, &100);
    assert_eq!(client.balance(&user1), 600);
    assert_eq!(client.balance(&user2), 400);
    assert_eq!(client.allowance(&user1, &user2), 100);
}
