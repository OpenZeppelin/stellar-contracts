#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use super::FungibleBurnable;
use crate::fungible::{FTBase, FungibleToken};

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        FTBase::internal_mint(&e, &account, 100);
        FTBase::burn(&e, &account, 50);
        assert_eq!(FTBase::balance(&e, &account), 50);
        assert_eq!(FTBase::total_supply(&e), 50);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_fungible_mint(&account, 100);
        event_assert.assert_fungible_burn(&account, 50);
    });
}

#[test]
fn burn_with_allowance_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        FTBase::internal_mint(&e, &owner, 100);
        FTBase::approve(&e, &owner, &spender, 30, 1000);
        FTBase::burn_from(&e, &spender, &owner, 30);
        assert_eq!(FTBase::balance(&e, &owner), 70);
        assert_eq!(FTBase::balance(&e, &spender), 0);
        assert_eq!(FTBase::total_supply(&e), 70);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_fungible_mint(&owner, 100);
        event_assert.assert_fungible_approve(&owner, &spender, 30, 1000);
        event_assert.assert_fungible_burn(&owner, 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn burn_with_insufficient_balance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        FTBase::internal_mint(&e, &account, 100);
        assert_eq!(FTBase::balance(&e, &account), 100);
        assert_eq!(FTBase::total_supply(&e), 100);
        FTBase::burn(&e, &account, 101);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn burn_with_no_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        FTBase::internal_mint(&e, &owner, 100);
        assert_eq!(FTBase::balance(&e, &owner), 100);
        assert_eq!(FTBase::total_supply(&e), 100);
        FTBase::burn_from(&e, &spender, &owner, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn burn_with_insufficient_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        FTBase::internal_mint(&e, &owner, 100);
        FTBase::approve(&e, &owner, &spender, 50, 100);
        assert_eq!(FTBase::allowance(&e, &owner, &spender), 50);
        assert_eq!(FTBase::balance(&e, &owner), 100);
        assert_eq!(FTBase::total_supply(&e), 100);
        FTBase::burn_from(&e, &spender, &owner, 60);
    });
}
