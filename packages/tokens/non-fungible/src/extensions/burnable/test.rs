#![allow(unused_variables)]
#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_event_assertion::EventAssertion;

use crate::{
    approve_for_all,
    extensions::{
        burnable::storage::{burn, burn_from},
        mintable::mint,
    },
    storage::{approve, balance},
    NonFungibleToken,
};

#[contract]
struct MockContract;

#[contractimpl]
impl NonFungibleToken for MockContract {
    fn balance(e: &Env, owner: Address) -> u32 {
        crate::storage2::balance::<Self>(e, &owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        crate::storage2::owner_of::<Self>(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        unimplemented!()
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        unimplemented!()
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        unimplemented!()
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        unimplemented!()
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        crate::storage2::get_approved::<Self>(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        crate::storage2::is_approved_for_all::<Self>(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        unimplemented!()
    }

    fn symbol(e: &Env) -> String {
        unimplemented!()
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        unimplemented!()
    }
}

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        burn::<MockContract>(&e, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

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
        let token_id = mint::<MockContract>(&e, &owner, 0);

        approve(&e, &owner, &spender, token_id, 1000);
        burn_from::<MockContract>(&e, &spender, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

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
        let token_id = mint::<MockContract>(&e, &owner, 0);

        approve_for_all(&e, &owner, &operator, 1000);

        burn_from::<MockContract>(&e, &operator, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

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
        let token_id = mint::<MockContract>(&e, &owner, 0);

        burn_from::<MockContract>(&e, &owner, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

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
        let token_id = mint::<MockContract>(&e, &owner, 0);

        burn::<MockContract>(&e, &spender, token_id);
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
        let token_id = mint::<MockContract>(&e, &owner, 0);

        burn_from::<MockContract>(&e, &spender, &owner, token_id);
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
        let _token_id = mint::<MockContract>(&e, &owner, 0);

        burn::<MockContract>(&e, &owner, non_existent_token_id);
    });
}
