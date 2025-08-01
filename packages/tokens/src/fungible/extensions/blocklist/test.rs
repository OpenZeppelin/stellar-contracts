extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use super::FungibleBlockList;
use crate::fungible::{
    extensions::{blocklist::storage::BlockList, burnable::FungibleBurnable},
    FTBase, FungibleToken,
};

#[contract]
struct MockContract;

impl FungibleBlockList for MockContract {
    type Impl = BlockList;
}

impl FungibleBurnable for MockContract {
    type Impl = FTBase;

    fn burn(e: &Env, from: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from]);
        Self::Impl::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from]);
        Self::Impl::burn_from(e, spender, from, amount);
    }
}

impl FungibleToken for MockContract {
    type Impl = FTBase;

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Self::assert_not_blocked(e, &[owner]);
        Self::Impl::approve(e, owner, spender, amount, live_until_ledger);
    }
}

#[test]
fn block_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Check initial state
        assert!(!MockContract::blocked(&e, &user));

        // Block user
        MockContract::block_user(&e, &user, &user);

        // Verify user is blocked
        assert!(MockContract::blocked(&e, &user));
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
        MockContract::block_user(&e, &user, &user);
        assert!(MockContract::blocked(&e, &user));

        // Unblock user
        MockContract::unblock_user(&e, &user, &user);

        // Verify user is not blocked
        assert!(!MockContract::blocked(&e, &user));
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
        MockContract::internal_mint(&e, &user1, 100);

        // Transfer tokens from user1 to user2
        MockContract::transfer(&e, &user1, &user2, 50);

        // Verify balances
        assert_eq!(MockContract::balance(&e, &user1), 50);
        assert_eq!(MockContract::balance(&e, &user2), 50);
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
        MockContract::internal_mint(&e, &user, 100);

        // Burn tokens from user
        MockContract::burn(&e, &user, 50);

        // Verify balance
        assert_eq!(MockContract::balance(&e, &user), 50);
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
        MockContract::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        MockContract::approve(&e, &user1, &user2, 50, 100);

        // Burn tokens from user1 by user2
        MockContract::burn_from(&e, &user2, &user1, 50);

        // Verify balance
        assert_eq!(MockContract::balance(&e, &user1), 50);
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
        MockContract::block_user(&e, &user1, &user1);

        // Mint tokens to user1
        MockContract::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 (blocked) to user2
        MockContract::transfer(&e, &user1, &user2, 50);
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
        MockContract::block_user(&e, &user2, &user2);

        // Mint tokens to user1
        MockContract::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 to user2 (blocked)
        MockContract::transfer(&e, &user1, &user2, 50);
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
        MockContract::block_user(&e, &user1, &user1);

        // Try to approve tokens from user1 (blocked) to user2
        MockContract::approve(&e, &user1, &user2, 50, 100);
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
        MockContract::block_user(&e, &user, &user);

        // Mint tokens to user
        MockContract::internal_mint(&e, &user, 100);

        // Try to burn tokens from user (blocked)
        MockContract::burn(&e, &user, 50);
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
        MockContract::block_user(&e, &user1, &user1);

        // Mint tokens to user1
        MockContract::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        MockContract::approve(&e, &user1, &user2, 50, 100);

        // Try to burn tokens from user1 by user2 (blocked)
        MockContract::burn_from(&e, &user2, &user1, 50);
    });
}
