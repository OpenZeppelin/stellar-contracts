#![allow(unused_variables)]
#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, Env, Map, String,
};
use stellar_event_assertion::EventAssertion;

use crate::{
    mintable::mint,
    storage::{
        approve, approve_for_all, balance, get_approved, is_approved_for_all, owner_of, transfer,
        update, StorageKey,
    },
    transfer_from, ApprovalForAllData, NonFungibleToken,
};

#[contract]
struct MockContract;

#[contractimpl]
impl NonFungibleToken for MockContract {
    fn balance(e: &Env, owner: Address) -> u32 {
        crate::storage::balance::<Self>(e, &owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        crate::storage::owner_of::<Self>(e, token_id)
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
        crate::storage::get_approved::<Self>(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        crate::storage::is_approved_for_all::<Self>(e, &owner, &operator)
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
fn approve_for_all_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        approve_for_all::<MockContract>(&e, &owner, &operator, 1000);

        let is_approved = is_approved_for_all::<MockContract>(&e, &owner, &operator);
        assert!(is_approved);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_approve_for_all(&owner, &operator, 1000);
    });
}

#[test]
fn revoke_approve_for_all_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        // set a pre-existing approve_for_all for the operator
        let key = StorageKey::ApprovalForAll(owner.clone());
        let mut approval_data = ApprovalForAllData { operators: Map::new(&e) };
        approval_data.operators.set(operator.clone(), 1000);

        e.storage().temporary().set(&key, &approval_data);

        let is_approved = is_approved_for_all::<MockContract>(&e, &owner, &operator);
        assert!(is_approved);

        // revoke approval
        approve_for_all::<MockContract>(&e, &owner, &operator, 0);
        let is_approved = is_approved_for_all::<MockContract>(&e, &owner, &operator);
        assert!(!is_approved);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_approve_for_all(&owner, &operator, 0);
    });
}

#[test]
fn approve_nft_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let approved = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);

        approve::<MockContract>(&e, &owner, &approved, token_id, 1000);

        let approved_address = get_approved::<MockContract>(&e, token_id);
        assert_eq!(approved_address, Some(approved.clone()));

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(1);
        event_assert.assert_non_fungible_approve(&owner, &approved, token_id, 1000);
    });
}

#[test]
fn approve_with_operator_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);
    let approved = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);

        approve_for_all::<MockContract>(&e, &owner, &operator, 1000);

        // approver is the operator on behalf of the owner
        approve::<MockContract>(&e, &operator, &approved, token_id, 1000);

        let approved_address = get_approved::<MockContract>(&e, token_id);
        assert_eq!(approved_address, Some(approved.clone()));

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_approve_for_all(&owner, &operator, 1000);
        event_assert.assert_non_fungible_approve(&operator, &approved, token_id, 1000);
    });
}

#[test]
fn transfer_nft_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        transfer::<MockContract>(&e, &owner, &recipient, token_id);

        assert_eq!(balance::<MockContract>(&e, &owner), 0);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);
        assert_eq!(owner_of::<MockContract>(&e, token_id), recipient);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
fn transfer_from_nft_approved_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Approve the spender
        approve::<MockContract>(&e, &owner, &spender, token_id, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from::<MockContract>(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance::<MockContract>(&e, &owner), 0);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);
        assert_eq!(owner_of::<MockContract>(&e, token_id), recipient);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_approve(&owner, &spender, token_id, 1000);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
fn transfer_from_nft_operator_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Approve the spender
        approve_for_all::<MockContract>(&e, &owner, &spender, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from::<MockContract>(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance::<MockContract>(&e, &owner), 0);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);
        assert_eq!(owner_of::<MockContract>(&e, token_id), recipient);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(3);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_approve_for_all(&owner, &spender, 1000);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
fn transfer_from_nft_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Attempt to transfer from the owner without approval
        transfer_from::<MockContract>(&e, &owner, &owner, &recipient, token_id);

        assert_eq!(balance::<MockContract>(&e, &owner), 0);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);
        assert_eq!(owner_of::<MockContract>(&e, token_id), recipient);

        let event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn transfer_nft_invalid_owner_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let unauthorized = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Attempt to transfer without authorization
        transfer::<MockContract>(&e, &unauthorized, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn transfer_from_nft_insufficient_approval_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Attempt to transfer from the owner without approval
        transfer_from::<MockContract>(&e, &spender, &owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn owner_of_non_existent_token_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let non_existent_token_id = 1;

    e.as_contract(&address, || {
        // Attempt to get the owner of a non-existent token
        owner_of::<MockContract>(&e, non_existent_token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")]
fn approve_with_invalid_live_until_ledger_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let approved = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        e.ledger().set_sequence_number(10);

        // Attempt to approve with an invalid live_until_ledger
        approve::<MockContract>(&e, &owner, &approved, token_id, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #303)")]
fn approve_with_invalid_approver_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let invalid_approver = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Attempt to approve with an invalid approver
        approve::<MockContract>(&e, &invalid_approver, &owner, token_id, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #305)")]
fn update_with_math_overflow_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        e.storage().persistent().set(&StorageKey::Balance(recipient.clone()), &u32::MAX);

        // Attempt to update which would cause a math overflow
        update::<MockContract>(&e, Some(&owner), Some(&recipient), token_id);
    });
}

#[test]
fn balance_of_non_existent_account_is_zero() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let non_existent_account = Address::generate(&e);

    e.as_contract(&address, || {
        // Check balance of a non-existent account
        let balance_value = balance::<MockContract>(&e, &non_existent_account);
        assert_eq!(balance_value, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn transfer_from_incorrect_owner_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let incorrect_owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Approve the spender
        approve::<MockContract>(&e, &owner, &spender, token_id, 1000);

        // Attempt to transfer from an incorrect owner
        transfer_from::<MockContract>(&e, &spender, &incorrect_owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn transfer_from_unauthorized_spender_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let unauthorized_spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = mint::<MockContract>(&e, &owner, 0);

        // Attempt to transfer from the owner using an unauthorized spender
        transfer_from::<MockContract>(&e, &unauthorized_spender, &owner, &recipient, token_id);
    });
}
