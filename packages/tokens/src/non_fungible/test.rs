#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};
use stellar_event_assertion::EventAssertion;

use crate::non_fungible::{Base, NFTStorageKey};

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
        Base::set_metadata(&e, base_uri, collection_name.clone(), collection_symbol.clone());

        let token_id = 4294967295;
        e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);
        let uri = Base::token_uri(&e, token_id);

        assert_eq!(uri, String::from_str(&e, "https://smth.com/4294967295"));
        assert_eq!(collection_name, Base::name(&e));
        assert_eq!(collection_symbol, Base::symbol(&e));

        // case token_id == 0
        let token_id = 0;
        e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);
        let uri = Base::token_uri(&e, token_id);

        assert_eq!(uri, String::from_str(&e, "https://smth.com/0"));

        // case empty string as base_uri
        let empty_base_uri = String::from_str(&e, "");
        Base::set_metadata(&e, empty_base_uri, collection_name, collection_symbol);
        let empty_uri = Base::token_uri(&e, token_id);

        assert_eq!(empty_uri, String::from_str(&e, ""));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #210)")]
fn get_metadata_should_panic_when_metadata_is_not_set() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let _ = Base::get_metadata(&e);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #211)")]
fn set_metadata_should_panic_when_base_uri_exceeds_max_length() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Create a base_uri that exceeds MAX_BASE_URI_LEN (200 characters)
        let too_long_base_uri = String::from_str(&e, &"a".repeat(201));
        let name = String::from_str(&e, "Test Collection");
        let symbol = String::from_str(&e, "TEST");

        // This should panic with BaseUriMaxLenExceeded error
        Base::set_metadata(&e, too_long_base_uri, name, symbol);
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
        Base::approve_for_all(&e, &owner, &operator, 1000);

        let is_approved = Base::is_approved_for_all(&e, &owner, &operator);
        assert!(is_approved);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let key = NFTStorageKey::ApprovalForAll(owner.clone(), operator.clone());

        e.storage().temporary().set(&key, &(1000_u32));

        let is_approved = Base::is_approved_for_all(&e, &owner, &operator);
        assert!(is_approved);

        // revoke approval
        Base::approve_for_all(&e, &owner, &operator, 0);

        let is_approved = Base::is_approved_for_all(&e, &owner, &operator);
        assert!(!is_approved);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);

        Base::approve(&e, &owner, &approved, token_id, 1000);

        let approved_address = Base::get_approved(&e, token_id);
        assert_eq!(approved_address, Some(approved.clone()));

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        e.storage().persistent().set(&NFTStorageKey::Owner(token_id), &owner);

        Base::approve_for_all(&e, &owner, &operator, 1000);

        // approver is the operator on behalf of the owner
        Base::approve(&e, &operator, &approved, token_id, 1000);

        let approved_address = Base::get_approved(&e, token_id);
        assert_eq!(approved_address, Some(approved.clone()));

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let token_id = Base::sequential_mint(&e, &owner);

        Base::transfer(&e, &owner, &recipient, token_id);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(Base::balance(&e, &recipient), 1);
        assert_eq!(Base::owner_of(&e, token_id), recipient);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let token_id = Base::sequential_mint(&e, &owner);

        // Approve the spender
        Base::approve(&e, &owner, &spender, token_id, 1000);

        // Transfer from the owner using the spender's approval
        Base::transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(Base::balance(&e, &recipient), 1);
        assert_eq!(Base::owner_of(&e, token_id), recipient);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let token_id = Base::sequential_mint(&e, &owner);

        // Approve the spender
        Base::approve_for_all(&e, &owner, &spender, 1000);

        // Transfer from the owner using the spender's approval
        Base::transfer_from(&e, &spender, &owner, &recipient, token_id);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(Base::balance(&e, &recipient), 1);
        assert_eq!(Base::owner_of(&e, token_id), recipient);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let token_id = Base::sequential_mint(&e, &owner);

        // Attempt to transfer from the owner without approval
        Base::transfer_from(&e, &owner, &owner, &recipient, token_id);

        assert_eq!(Base::balance(&e, &owner), 0);
        assert_eq!(Base::balance(&e, &recipient), 1);
        assert_eq!(Base::owner_of(&e, token_id), recipient);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id);
        event_assert.assert_non_fungible_transfer(&owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn transfer_nft_invalid_owner_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let unauthorized = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        // Attempt to transfer without authorization
        Base::transfer(&e, &unauthorized, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn transfer_from_nft_insufficient_approval_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        // Attempt to transfer from the owner without approval
        Base::transfer_from(&e, &spender, &owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")]
fn owner_of_non_existent_token_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let non_existent_token_id = 1;

    e.as_contract(&address, || {
        // Attempt to get the owner of a non-existent token
        Base::owner_of(&e, non_existent_token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #204)")]
fn approve_with_invalid_live_until_ledger_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let approved = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        e.ledger().set_sequence_number(10);

        // Attempt to approve with an invalid live_until_ledger
        Base::approve(&e, &owner, &approved, token_id, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #203)")]
fn approve_with_invalid_approver_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let invalid_approver = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        // Attempt to approve with an invalid approver
        Base::approve(&e, &invalid_approver, &owner, token_id, 1000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #205)")]
fn update_with_math_overflow_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        e.storage().persistent().set(&NFTStorageKey::Balance(recipient.clone()), &u32::MAX);

        // Attempt to update which would cause a math overflow
        Base::update(&e, Some(&owner), Some(&recipient), token_id);
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
        let balance_value = Base::balance(&e, &non_existent_account);
        assert_eq!(balance_value, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")]
fn transfer_from_incorrect_owner_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let incorrect_owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        // Approve the spender
        Base::approve(&e, &owner, &spender, token_id, 1000);

        // Attempt to transfer from an incorrect owner
        Base::transfer_from(&e, &spender, &incorrect_owner, &recipient, token_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")]
fn transfer_from_unauthorized_spender_fails() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let owner = Address::generate(&e);
    let unauthorized_spender = Address::generate(&e);
    let recipient = Address::generate(&e);

    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &owner);

        // Attempt to transfer from the owner using an unauthorized spender
        Base::transfer_from(&e, &unauthorized_spender, &owner, &recipient, token_id);
    });
}

#[test]
fn mint_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let account = Address::generate(&e);
    e.as_contract(&address, || {
        let token_id = Base::sequential_mint(&e, &account);
        assert_eq!(Base::balance(&e, &account), 1);

        let mut event_assert = EventAssertion::new(&e, address.clone());
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
        let token_id1 = Base::sequential_mint(&e, &owner);
        let token_id2 = Base::sequential_mint(&e, &owner);

        let mut event_assert = EventAssertion::new(&e, address.clone());
        event_assert.assert_event_count(2);
        event_assert.assert_non_fungible_mint(&owner, token_id1);
        event_assert.assert_non_fungible_mint(&owner, token_id2);
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
        Base::sequential_mint(&e, &account);
        assert_eq!(Base::balance(&e, &account), 1);
    });
}
