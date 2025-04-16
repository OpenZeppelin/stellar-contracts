#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    extensions::{
        capped::{check_cap, query_cap, set_cap},
        mintable::mint,
    },
    storage::{balance, total_supply},
};

#[contract]
struct MockContract;

#[test]
fn test_mint_under_cap() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
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
    let contract_address = e.register(MockContract, ());
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
    let contract_address = e.register(MockContract, ());
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
    let contract_address = e.register(MockContract, ());
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
#[should_panic(expected = "Error(Contract, #204)")]
fn test_check_cap_overflows() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 170_141_183_460_469_231_731_687_303_715_884_105_727); // MAX value for i128;
        mint(&e, &user, 170_141_183_460_469_231_731_687_303_715_884_105_727);

        check_cap(&e, 1); // should overflow
    });
}

#[test]
fn test_query_cap() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        let cap = query_cap(&e);
        assert_eq!(cap, 1000);
    });
}
