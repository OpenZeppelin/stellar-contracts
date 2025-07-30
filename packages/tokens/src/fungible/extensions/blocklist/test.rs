extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use super::{FungibleBlockList, FungibleBlockListExt};
use crate::fungible::{
    extensions::{blocklist::storage::BlockList, burnable::FungibleBurnable},
    FTBase, FungibleToken,
};

#[contract]
struct MockContract;

type BurableBlockList = FungibleBlockListExt<BlockList, FTBase>;
type FungibleTokenBlockList = FungibleBlockListExt<BlockList, FTBase>;

#[test]
fn block_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Check initial state
        assert!(!BlockList::blocked(&e, &user));

        // Block user
        BlockList::block_user(&e, &user, &user);

        // Verify user is blocked
        assert!(BlockList::blocked(&e, &user));
    });
}

#[test]
fn unblock_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user first
        BlockList::block_user(&e, &user, &user);
        assert!(BlockList::blocked(&e, &user));

        // Unblock user
        BlockList::unblock_user(&e, &user, &user);

        // Verify user is not blocked
        assert!(!BlockList::blocked(&e, &user));
    });
}

#[test]
fn transfer_with_unblocked_users_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Transfer tokens from user1 to user2
        FungibleTokenBlockList::transfer(&e, &user1, &user2, 50);

        // Verify balances
        assert_eq!(FTBase::balance(&e, &user1), 50);
        assert_eq!(FTBase::balance(&e, &user2), 50);
    });
}

#[test]
fn blocklist_burn_override_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Mint tokens to user
        FTBase::internal_mint(&e, &user, 100);

        // Burn tokens from user
        BurableBlockList::burn(&e, &user, 50);

        // Verify balance
        assert_eq!(FTBase::balance(&e, &user), 50);
    });
}

#[test]
fn blocklist_burn_from_override_works() {
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

        // Burn tokens from user1 by user2
        BurableBlockList::burn_from(&e, &user2, &user1, 50);

        // Verify balance
        assert_eq!(FTBase::balance(&e, &user1), 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn transfer_with_sender_blocked_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user1
        BlockList::block_user(&e, &user1, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 (blocked) to user2
        FungibleTokenBlockList::transfer(&e, &user1, &user2, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn transfer_with_receiver_blocked_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user2
        BlockList::block_user(&e, &user2, &user2);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 to user2 (blocked)
        FungibleTokenBlockList::transfer(&e, &user1, &user2, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn approve_with_owner_blocked_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user1
        BlockList::block_user(&e, &user1, &user1);

        // Try to approve tokens from user1 (blocked) to user2
        FungibleTokenBlockList::approve(&e, &user1, &user2, 50, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn burn_with_blocked_user_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user
        BlockList::block_user(&e, &user, &user);

        // Mint tokens to user
        FTBase::internal_mint(&e, &user, 100);

        // Try to burn tokens from user (blocked)
        BurableBlockList::burn(&e, &user, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #114)")]
fn burn_from_with_blocked_user_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Block user1
        BlockList::block_user(&e, &user1, &user1);

        // Mint tokens to user1
        FTBase::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        FTBase::approve(&e, &user1, &user2, 50, 100);

        // Try to burn tokens from user1 by user2 (blocked)
        BurableBlockList::burn_from(&e, &user2, &user1, 50);
    });
}
