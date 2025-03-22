#![allow(unused_variables)]
#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};
use stellar_event_assertion::EventAssertion;

use crate::{extensions::mintable::storage::mint, storage::balance, NonFungibleToken};

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
fn mint_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &account, 0);
        assert_eq!(balance(&e, &account), 1);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_non_fungible_mint(&account, token_id);
    });
}

/// Test that confirms the base mint implementation does NOT require
/// authorization
///
/// IMPORTANT: This test verifies the intentional design choice that the base
/// mint implementation doesn't include authorization controls. This is NOT a
/// security flaw but rather a design decision to give implementers flexibility
/// in how they implement authorization.
///
/// When using this function in your contracts, you MUST add your own
/// authorization controls to ensure only designated accounts can mint tokens.
#[test]
fn mint_base_implementation_has_no_auth() {
    let e = Env::default();
    // Note: we're intentionally NOT mocking any auths
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);

    // This should NOT panic even without authorization
    e.as_contract(&address, || {
        mint::<MockContract>(&e, &account, 0);
        assert_eq!(balance(&e, &account), 1);
    });
}
