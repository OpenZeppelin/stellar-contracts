extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{extensions::allowlist::storage::AllowList, Base};

#[contract]
struct MockContract;

#[test]
fn allow_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Check initial state
        assert!(!AllowList::allowed(&e, &user));

        // Allow user
        AllowList::allow_user(&e, &user);

        // Verify user is allowed
        assert!(AllowList::allowed(&e, &user));
    });
}

#[test]
fn disallow_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow user first
        AllowList::allow_user(&e, &user);
        assert!(AllowList::allowed(&e, &user));
    });

    e.as_contract(&address, || {
        // Disallow user
        AllowList::disallow_user(&e, &user);

        // Verify user is not allowed
        assert!(!AllowList::allowed(&e, &user));
    });
}

#[test]
fn transfer_with_allowed_users_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow both users
        AllowList::allow_user(&e, &user1);
        AllowList::allow_user(&e, &user2);

        // Mint tokens to user1
        Base::mint(&e, &user1, 100);

        // Transfer tokens from user1 to user2
        AllowList::transfer(&e, &user1, &user2, 50);

        // Verify balances
        assert_eq!(Base::balance(&e, &user1), 50);
        assert_eq!(Base::balance(&e, &user2), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn transfer_with_sender_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow only user2
        AllowList::allow_user(&e, &user2);

        // Mint tokens to user1
        Base::mint(&e, &user1, 100);

        // Try to transfer tokens from user1 (not allowed) to user2
        AllowList::transfer(&e, &user1, &user2, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn transfer_with_receiver_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow only user1
        AllowList::allow_user(&e, &user1);

        // Mint tokens to user1
        Base::mint(&e, &user1, 100);

        // Try to transfer tokens from user1 to user2 (not allowed)
        AllowList::transfer(&e, &user1, &user2, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn approve_with_owner_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Try to approve tokens from user1 (not allowed) to user2 (not allowed)
        AllowList::approve(&e, &user1, &user2, 50, 100);
    });
}
