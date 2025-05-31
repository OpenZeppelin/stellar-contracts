#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    extensions::capped::{check_cap, query_cap, set_cap},
    Base,
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
        Base::mint(&e, &user, 500);

        assert_eq!(Base::balance(&e, &user), 500);
        assert_eq!(Base::total_supply(&e), 500);
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
        Base::mint(&e, &user, 1000);

        assert_eq!(Base::balance(&e, &user), 1000);
        assert_eq!(Base::total_supply(&e), 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn test_mint_exceeds_cap() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        check_cap(&e, 1001);
        Base::mint(&e, &user, 1001); // This should panic
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn test_mint_multiple_exceeds_cap() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, 1000);

        // Mint 600 tokens first
        check_cap(&e, 600);
        Base::mint(&e, &user, 600);

        assert_eq!(Base::balance(&e, &user), 600);
        assert_eq!(Base::total_supply(&e), 600);

        // Attempt to mint 500 more tokens (would exceed cap)
        check_cap(&e, 500);
        Base::mint(&e, &user, 500); // This should panic
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #104)")]
fn test_check_cap_overflows() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_cap(&e, i128::MAX);
        Base::mint(&e, &user, i128::MAX);

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

#[test]
#[should_panic(expected = "Error(Contract, #107)")]
fn test_invalid_cap() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        // Attempt to set a negative cap value, which should trigger InvalidCap error
        set_cap(&e, -100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]
fn test_cap_not_set() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        // Try to query cap without setting it first, which should trigger CapNotSet
        // error
        let _ = query_cap(&e);
    });
}
