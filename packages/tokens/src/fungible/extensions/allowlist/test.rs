extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::fungible::{
    extensions::{
        allowlist::{AllowList, FungibleAllowList, FungibleAllowListExt},
        burnable::FungibleBurnable,
    },
    FTBase, FungibleToken,
};

type BurableAllowList = FungibleAllowListExt<AllowList, FTBase>;
type FungibleTokenAllowList = FungibleAllowListExt<AllowList, FTBase>;

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
        AllowList::allow_user(&e, &user, &user);

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
        AllowList::allow_user(&e, &user, &user);
        assert!(AllowList::allowed(&e, &user));
    });

    e.as_contract(&address, || {
        // Disallow user
        AllowList::disallow_user(&e, &user, &user);

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
        AllowList::allow_user(&e, &user1, &user1);
        AllowList::allow_user(&e, &user2, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Transfer tokens from user1 to user2
        FungibleTokenAllowList::transfer(&e, &user1, &user2, 50);

        // Verify balances
        assert_eq!(FTBase::balance(&e, &user1), 50);
        assert_eq!(FTBase::balance(&e, &user2), 50);
    });
}

#[test]
fn allowlist_burn_override_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow user first
        AllowList::allow_user(&e, &user, &user);

        // Mint tokens to user
        FTBase::internal_mint(&e, &user, 100);

        // Burn tokens from user
        BurableAllowList::burn(&e, &user, 50);

        // Verify balance
        assert_eq!(FTBase::balance(&e, &user), 50);
    });
}

#[test]
fn allowlist_burn_from_override_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow user1 first
        AllowList::allow_user(&e, &user1, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        FTBase::approve(&e, &user1, &user2, 50, 100);

        // Burn tokens from user1 by user2
        BurableAllowList::burn_from(&e, &user2, &user1, 50);

        // Verify balance
        assert_eq!(FTBase::balance(&e, &user1), 50);
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
        AllowList::allow_user(&e, &user2, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 (not allowed) to user2
        FungibleTokenAllowList::transfer(&e, &user1, &user2, 50);
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
        AllowList::allow_user(&e, &user1, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 to user2 (not allowed)
        FungibleTokenAllowList::transfer(&e, &user1, &user2, 50);
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
        FungibleTokenAllowList::approve(&e, &user1, &user2, 50, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn burn_with_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint tokens to user
        FTBase::internal_mint(&e, &user, 100);

        // Try to burn tokens from user (not allowed)
        BurableAllowList::burn(&e, &user, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn burn_from_with_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        FTBase::approve(&e, &user1, &user2, 50, 100);

        // Try to burn tokens from user1 by user2 (not allowed)
        BurableAllowList::burn_from(&e, &user2, &user1, 50);
    });
}
