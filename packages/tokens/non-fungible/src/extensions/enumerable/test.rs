use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    Address, Env, Vec,
};

use crate::storage::StorageKey;
use super::{self as enumerable, NonFungibleEnumerable};

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