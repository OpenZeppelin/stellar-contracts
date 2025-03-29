#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::{
    extensions::burnable::storage::{burn, burn_from},
    storage::{allowance, approve, balance, mint, total_supply},
};

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        burn(&e, &account, 50);
        assert_eq!(balance(&e, &account), 50);
        assert_eq!(total_supply(&e), 50);

        let event_assert = EventAssertion::new(&e, address.clone());
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
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 30, 1000);
        burn_from(&e, &spender, &owner, 30);
        assert_eq!(balance(&e, &owner), 70);
        assert_eq!(balance(&e, &spender), 0);
        assert_eq!(total_supply(&e), 70);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_fungible_mint(&owner, 100);
        event_assert.assert_fungible_approve(&owner, &spender, 30, 1000);
        event_assert.assert_fungible_burn(&owner, 30);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")]
fn burn_with_insufficient_balance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &account, 100);
        assert_eq!(balance(&e, &account), 100);
        assert_eq!(total_supply(&e), 100);
        burn(&e, &account, 101);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn burn_with_no_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        assert_eq!(balance(&e, &owner), 100);
        assert_eq!(total_supply(&e), 100);
        burn_from(&e, &spender, &owner, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn burn_with_insufficient_allowance_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, 50, 100);
        assert_eq!(allowance(&e, &owner, &spender), 50);
        assert_eq!(balance(&e, &owner), 100);
        assert_eq!(total_supply(&e), 100);
        burn_from(&e, &spender, &owner, 60);
    });
}

#[test]
fn burn_requires_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let from = Address::generate(&e);
    let amount = 50;

    e.as_contract(&address, || {
        mint(&e, &from, 100);
        burn(&e, &from, amount);
    });

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _invocation) = &auths[0];
    assert_eq!(addr, &from);
    // assert_eq!(
    //     invocation.function,
    //     AuthorizedFunction::Contract((
    //         address.clone(),
    //         symbol_short!("burn"),
    //         vec![&e, from.clone().into_val(&e), amount.into_val(&e)]
    //     ))
    // );
}

#[test]
fn burn_from_requires_auth() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let amount = 50;

    e.as_contract(&address, || {
        mint(&e, &owner, 100);
        approve(&e, &owner, &spender, amount, 1000);
        burn_from(&e, &spender, &owner, amount);
    });

    let auths = e.auths();
    assert_eq!(auths.len(), 2);
    // Verify approve auth
    let (addr, _invocation) = &auths[0];
    assert_eq!(addr, &owner);
    // assert_eq!(
    //     invocation.function,
    //     AuthorizedFunction::Contract((
    //         address.clone(),
    //         symbol_short!("approve"),
    //         vec![
    //             &e,
    //             owner.clone().into_val(&e),
    //             spender.clone().into_val(&e),
    //             amount.into_val(&e),
    //             1000.into_val(&e)
    //         ]
    //     ))
    // );
    // Verify burn_from auth
    let (addr, _invocation) = &auths[1];
    assert_eq!(addr, &spender);
    // assert_eq!(
    //     invocation.function,
    //     AuthorizedFunction::Contract((
    //         address.clone(),
    //         symbol_short!("burn_from"),
    //         vec![
    //             &e,
    //             spender.clone().into_val(&e),
    //             owner.clone().into_val(&e),
    //             amount.into_val(&e)
    //         ]
    //     ))
    // );
}
