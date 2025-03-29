#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::Base;

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
fn burn_from_with_approve_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::approve(&e, &owner, &spender, token_id, 1000);
        Base::burn_from(&e, &spender, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_approve(&owner, &spender, token_id, 1000);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
fn burn_from_with_operator_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::approve_for_all(&e, &owner, &operator, 1000);

        Base::burn_from(&e, &operator, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_approve_for_all(&owner, &operator, 1000);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
fn burn_from_with_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn_from(&e, &owner, &owner, token_id);

        assert!(Base::balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_burn(&owner, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn burn_with_not_owner_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &spender, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn burn_from_with_insufficient_approval_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        Base::burn_from(&e, &spender, &owner, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn burn_with_non_existent_token_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let non_existent_token_id = 2;

    e.as_contract(&address, || {
        let _token_id = Base::sequential_mint(&e, &owner);

        Base::burn(&e, &owner, non_existent_token_id);
    });
}
