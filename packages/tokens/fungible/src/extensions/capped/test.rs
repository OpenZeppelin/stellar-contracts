#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    extensions::{
        capped::storage::{check_cap, query_cap, set_cap},
        mintable::mint,
    },
    storage::{balance, total_supply},
};

// Mock contract for testing
#[contract]
struct TestToken;

#[test]
fn test_mint_under_cap() {
    let e = Env::default();
    let contract_address = e.register(TestToken, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        check_cap(&e, 500);
        mint(&e, &user, 500);

        assert_eq!(balance(&e, &user), 500);
        assert_eq!(total_supply(&e), 500);
    });
}

#[test]
fn test_mint_exact_cap() {
    let e = Env::default();
    let contract_address = e.register(TestToken, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        check_cap(&e, 1000);
        mint(&e, &user, 1000);

        assert_eq!(balance(&e, &user), 1000);
        assert_eq!(total_supply(&e), 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")]
fn test_mint_exceeds_cap() {
    let e = Env::default();
    let contract_address = e.register(TestToken, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        check_cap(&e, 1001);
        mint(&e, &user, 1001); // This should panic
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")]
fn test_mint_multiple_exceeds_cap() {
    let e = Env::default();
    let contract_address = e.register(TestToken, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        // Mint 600 tokens first
        check_cap(&e, 600);
        mint(&e, &user, 600);

        assert_eq!(balance(&e, &user), 600);
        assert_eq!(total_supply(&e), 600);

        // Attempt to mint 500 more tokens (would exceed cap)
        check_cap(&e, 500);
        mint(&e, &user, 500); // This should panic
    });
}

#[test]
fn test_query_cap() {
    let e = Env::default();
    let contract_address = e.register(TestToken, ());

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        let cap = query_cap(&e);
        assert_eq!(cap, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #207)")]
fn test_set_negative_cap() {
    let e = Env::default();
    e.as_contract(&e.register(TestToken, ()), || {
        // Attempt to set a negative cap
        set_cap(&e, -100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #208)")]
fn test_query_cap_not_set() {
    let e = Env::default();
    e.as_contract(&e.register(TestToken, ()), || {
        // Attempt to query cap that hasn't been set
        query_cap(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #208)")]
fn test_cap_not_set() {
    let e = Env::default();
    let token = e.register(TestToken, ());

    e.as_contract(&token, || {
        // First verify the cap is not set by querying total supply
        assert_eq!(total_supply(&e), 0);
        
        // Attempt to check cap without setting it first - should panic with CapNotSet (error code 208)
        check_cap(&e, 100);
        
        // The following should never execute due to the panic
        assert!(false, "This code should not be reached");
    });
}

#[test]
fn test_query_cap_direct_error_path() {
    let e = Env::default();
    e.as_contract(&e.register_contract(None, TestToken), || {
        // Check the cap is initially unset
        let result: Option<i128> = e.storage().instance().get(&crate::extensions::capped::storage::CAP_KEY);
        assert_eq!(result, None);
        
        // Set a cap
        e.storage().instance().set(&crate::extensions::capped::storage::CAP_KEY, &1000_i128);
        
        // Verify the cap was set correctly
        let result: Option<i128> = e.storage().instance().get(&crate::extensions::capped::storage::CAP_KEY);
        assert_eq!(result, Some(1000_i128));
    });
}
