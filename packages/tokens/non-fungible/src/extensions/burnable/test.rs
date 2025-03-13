#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};
use stellar_event_assertion::EventAssertion;

use crate::{
    extensions::burnable::storage::{burn, burn_from},
    set_approval_for_all,
    storage::{approve, balance},
    StorageKey,
};

#[contract]
struct MockContract;

#[test]
fn burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        burn(&e, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        // event_assert.assert_mint(&owner, 100);
        event_assert.assert_non_fungible_burn(&owner, 1);
    });
}

#[test]
fn burn_from_with_approve_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        approve(&e, &owner, &spender, token_id, 1000);

        // Attempt to transfer from the owner without approval
        burn_from(&e, &spender, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        // event_assert.assert_mint(&owner, 100);
        event_assert.assert_non_fungible_approve(&owner, &spender, 1, 1000);
        event_assert.assert_non_fungible_burn(&owner, 1);
    });
}

#[test]
fn burn_from_with_operator_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        set_approval_for_all(&e, &owner, &operator, true, 1000);

        // Attempt to transfer from the owner without approval
        burn_from(&e, &operator, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        // event_assert.assert_mint(&owner, 100);
        event_assert.assert_approve_for_all(&owner, &operator, true, 1000);
        event_assert.assert_non_fungible_burn(&owner, 1);
    });
}

#[test]
fn burn_from_with_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        burn_from(&e, &owner, &owner, token_id);

        assert!(balance(&e, &owner) == 0);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        // event_assert.assert_mint(&owner, 100);
        event_assert.assert_non_fungible_burn(&owner, 1);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        burn(&e, &spender, token_id);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        burn_from(&e, &spender, &owner, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn burn_with_non_existent_token_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let token_id = 1;
    let non_existent_token_id = 2;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        burn(&e, &owner, non_existent_token_id);
    });
}
