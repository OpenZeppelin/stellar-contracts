#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    Address, Env, Map, String,
};
use stellar_event_assertion::EventAssertion;

use crate::{
    mintable::sequential_mint,
    name,
    non_fungible::Balance,
    set_metadata,
    storage::{
        approve, approve_for_all, balance, get_approved, is_approved_for_all, owner_of, transfer,
        update, StorageKey,
    },
    symbol, token_uri, transfer_from, ApprovalForAllData,
};

#[contract]
struct MockContract;

#[test]
fn metadata_works() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);

    e.as_contract(&address, || {
        let base_uri = String::from_str(&e, "https://smth.com/");
        let collection_name = String::from_str(&e, "My NFT collection");
        let collection_symbol = String::from_str(&e, "NFT");
        set_metadata(&e, base_uri.clone(), collection_name.clone(), collection_symbol.clone());

        let token_id = 4294967295;
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        let uri = token_uri(&e, token_id);

        assert_eq!(uri, String::from_str(&e, "https://smth.com/4294967295"));
        assert_eq!(collection_name, name(&e));
        assert_eq!(collection_symbol, symbol(&e));
    });
}

#[test]
fn approve_for_all_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        approve_for_all(&e, &owner, &operator, 1000);

        let is_approved = is_approved_for_all(&e, &owner, &operator);
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

        let is_approved = is_approved_for_all(&e, &owner, &operator);
        assert!(is_approved);

        // revoke approval
        approve_for_all(&e, &owner, &operator, 0);
        let is_approved = is_approved_for_all(&e, &owner, &operator);
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

        approve(&e, &owner, &approved, token_id, 1000);

        let approved_address = get_approved(&e, token_id);
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

        approve_for_all(&e, &owner, &operator, 1000);

        // approver is the operator on behalf of the owner
        approve(&e, &operator, &approved, token_id, 1000);

        let approved_address = get_approved(&e, token_id);
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
        let token_id = sequential_mint(&e, &owner);

        transfer(&e, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);

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
        let token_id = sequential_mint(&e, &owner);

        // Approve the spender
        approve(&e, &owner, &spender, token_id, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);

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
        let token_id = sequential_mint(&e, &owner);

        // Approve the spender
        approve_for_all(&e, &owner, &spender, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);

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
        let token_id = sequential_mint(&e, &owner);

        // Attempt to transfer from the owner without approval
        transfer_from(&e, &owner, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);

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
        let token_id = sequential_mint(&e, &owner);

        // Attempt to transfer without authorization
        transfer(&e, &unauthorized, &recipient, token_id);
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
        let token_id = sequential_mint(&e, &owner);

        // Attempt to transfer from the owner without approval
        transfer_from(&e, &spender, &owner, &recipient, token_id);
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
        owner_of(&e, non_existent_token_id);
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
        let token_id = sequential_mint(&e, &owner);

        e.ledger().set_sequence_number(10);

        // Attempt to approve with an invalid live_until_ledger
        approve(&e, &owner, &approved, token_id, 1);
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
        let token_id = sequential_mint(&e, &owner);

        // Attempt to approve with an invalid approver
        approve(&e, &invalid_approver, &owner, token_id, 1000);
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
        let token_id = sequential_mint(&e, &owner);

        e.storage().persistent().set(&StorageKey::Balance(recipient.clone()), &Balance::MAX);

        // Attempt to update which would cause a math overflow
        update(&e, Some(&owner), Some(&recipient), token_id);
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
        let balance_value = balance(&e, &non_existent_account);
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
        let token_id = sequential_mint(&e, &owner);

        // Approve the spender
        approve(&e, &owner, &spender, token_id, 1000);

        // Attempt to transfer from an incorrect owner
        transfer_from(&e, &spender, &incorrect_owner, &recipient, token_id);
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
        let token_id = sequential_mint(&e, &owner);

        // Attempt to transfer from the owner using an unauthorized spender
        transfer_from(&e, &unauthorized_spender, &owner, &recipient, token_id);
    });
}
