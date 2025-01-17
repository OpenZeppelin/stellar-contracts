#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    Address, Env,
};

use crate::storage::{
    allowance, approve, balance, burn, mint, total_supply, transfer, transfer_from,
};

#[contract]
struct MockContract;

#[test]
fn initial_state() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        assert_eq!(total_supply(&e), 0);
        assert_eq!(balance(&e, &account), 0);
    });
}

#[test]
fn mint_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        assert_eq!(balance(&e, &account), 100);
        assert_eq!(total_supply(&e), 100);
    });
}

#[test]
fn burn_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        burn(&e, &account, 50);
        assert_eq!(balance(&e, &account), 50);
        assert_eq!(total_supply(&e), 50);
    });
}

#[test]
fn transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        transfer(&e, &from, &recipient, 50);
        assert_eq!(balance(&e, &from), 50);
        assert_eq!(balance(&e, &recipient), 50);

        let events = e.events().all();
        assert_eq!(events.len(), 1);
    });
}

#[test]
fn approve_and_transfer_from() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 50, 1000);

        let allowance_val = allowance(&e, &owner, &spender);
        assert_eq!(allowance_val, 50);

        transfer_from(&e, &spender, &owner, &recipient, 30);
        assert_eq!(balance(&e, &owner), 70);
        assert_eq!(balance(&e, &recipient), 30);

        let updated_allowance = allowance(&e, &owner, &spender);
        assert_eq!(updated_allowance, 20);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn errors_transfer_insufficient_balance() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &from, 50);
        transfer(&e, &from, &recipient, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn errors_transfer_from_insufficient_allowance() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 30, 1000);
        transfer_from(&e, &spender, &owner, &recipient, 50);
    });
}
