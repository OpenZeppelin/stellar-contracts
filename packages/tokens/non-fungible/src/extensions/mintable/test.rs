#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::{extensions::mintable::storage::sequential_mint, storage::balance};

#[contract]
struct MockContract;

#[test]
fn mint_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        let token_id = sequential_mint(&e, &account);
        assert_eq!(balance(&e, &account), 1);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_non_fungible_mint(&account, token_id);
    });
}

#[test]
fn test_counter_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id1 = sequential_mint(&e, &owner);
        let _token_id2 = sequential_mint(&e, &owner);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id1);

        // TODO: below fails because the same event is read by the
        // `event_assert`, not the next one. event_assert.
        // assert_non_fungible_mint(&owner, token_id2);
    });
}

/// Test that confirms the base mint implementation does NOT require
/// authorization
///
/// **IMPORTANT**: This test verifies the intentional design choice that the
/// base mint implementation doesn't include authorization controls. This is NOT
/// a security flaw but rather a design decision to give implementers
/// flexibility in how they implement authorization.
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
        sequential_mint(&e, &account);
        assert_eq!(balance(&e, &account), 1);
    });
}
