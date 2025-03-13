#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    Address, Env, Vec,
};

use crate::{
    storage::{
        approve, balance, get_approved, is_approved_for_all, owner_of, set_approval_for_all,
        transfer, update, StorageKey,
    },
    transfer_from,
    extensions::enumerable::{self, NonFungibleEnumerable},
};

#[contract]
struct MockContract;

#[contract]
pub struct MockEnumerableContract;

impl NonFungibleEnumerable for MockEnumerableContract {
    fn total_supply(e: &Env) -> u128 {
        enumerable::total_supply(e)
    }

    fn tokens_of_owner(e: &Env, owner: Address) -> Vec<u128> {
        enumerable::tokens_of_owner(e, &owner)
    }

    fn all_tokens(e: &Env) -> Vec<u128> {
        enumerable::all_tokens(e)
    }
}

#[test]
fn set_approval_for_all_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let operator = Address::generate(&e);

    e.as_contract(&address, || {
        set_approval_for_all(&e, &owner, &operator, true, 1000);

        let is_approved = is_approved_for_all(&e, &owner, &operator);
        assert!(is_approved);
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
    });
}

#[test]
// TODO:
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

        set_approval_for_all(&e, &owner, &operator, true, 1000);

        // approver is the operator on behalf of the owner
        approve(&e, &operator, &approved, token_id, 1000);

        let approved_address = get_approved(&e, token_id);
        assert_eq!(approved_address, Some(approved.clone()));
    });
}

#[test]
fn transfer_nft_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let token_id = 1u128;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        transfer(&e, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Approve the spender
        approve(&e, &owner, &spender, token_id, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Approve the spender
        set_approval_for_all(&e, &owner, &spender, true, 1000);

        // Transfer from the owner using the spender's approval
        transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);
    });
}

#[test]
fn transfer_from_nft_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner without approval
        transfer_from(&e, &owner, &owner, &recipient, token_id);

        assert_eq!(balance(&e, &owner), 0);
        assert_eq!(balance(&e, &recipient), 1);
        assert_eq!(owner_of(&e, token_id), recipient);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Attempt to get the owner of a non-existent token
        owner_of(&e, token_id);
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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);
        e.storage().persistent().set(&StorageKey::Balance(recipient.clone()), &u128::MAX);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

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
    let token_id = 1;

    e.as_contract(&address, || {
        // Mint the NFT by setting the owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        e.storage().persistent().set(&StorageKey::Balance(owner.clone()), &1u128);

        // Attempt to transfer from the owner using an unauthorized spender
        transfer_from(&e, &unauthorized_spender, &owner, &recipient, token_id);
    });
}

#[test]
fn test_enumerable_minting() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockEnumerableContract, ());
    let owner = Address::generate(&e);
    let token_id = 1u128;

    e.as_contract(&address, || {
        // Mint a token
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        enumerable::add_token(&e, token_id, &owner);

        // Check total supply
        assert_eq!(enumerable::total_supply(&e), 1);

        // Check owner's tokens
        let owner_tokens = enumerable::tokens_of_owner(&e, &owner);
        assert_eq!(owner_tokens.len(), 1);
        assert_eq!(owner_tokens.get(0).unwrap(), token_id);

        // Check all tokens
        let all_tokens = enumerable::all_tokens(&e);
        assert_eq!(all_tokens.len(), 1);
        assert_eq!(all_tokens.get(0).unwrap(), token_id);
    });
}

#[test]
fn test_enumerable_transfer() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockEnumerableContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let token_id = 1u128;

    e.as_contract(&address, || {
        // Mint a token
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        enumerable::add_token(&e, token_id, &owner);

        // Transfer the token
        enumerable::update_token_lists(&e, token_id, Some(&owner), Some(&recipient));

        // Check owner's tokens (should be empty)
        let owner_tokens = enumerable::tokens_of_owner(&e, &owner);
        assert_eq!(owner_tokens.len(), 0);

        // Check recipient's tokens
        let recipient_tokens = enumerable::tokens_of_owner(&e, &recipient);
        assert_eq!(recipient_tokens.len(), 1);
        assert_eq!(recipient_tokens.get(0).unwrap(), token_id);

        // Total supply should remain the same
        assert_eq!(enumerable::total_supply(&e), 1);
    });
}

#[test]
fn test_enumerable_burn() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockEnumerableContract, ());
    let owner = Address::generate(&e);
    let token_id = 1u128;

    e.as_contract(&address, || {
        // Mint a token
        e.storage().persistent().set(&StorageKey::Owner(token_id), &owner);
        enumerable::add_token(&e, token_id, &owner);

        // Burn the token
        enumerable::update_token_lists(&e, token_id, Some(&owner), None);

        // Check owner's tokens (should be empty)
        let owner_tokens = enumerable::tokens_of_owner(&e, &owner);
        assert_eq!(owner_tokens.len(), 0);

        // Check all tokens (should be empty)
        let all_tokens = enumerable::all_tokens(&e);
        assert_eq!(all_tokens.len(), 0);

        // Total supply should be 0
        assert_eq!(enumerable::total_supply(&e), 0);
    });
}
