extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::fungible::{
    extensions::{
        allowlist::{AllowList, FungibleAllowList},
        burnable::FungibleBurnable,
    },
    FTBase, FungibleToken,
};

impl FungibleToken for MockContract {
    type Impl = FTBase;

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::assert_allowed(e, &[from, to]);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::assert_allowed(e, &[from, to]);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Self::assert_allowed(e, &[owner]);
        Self::Impl::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl FungibleBurnable for MockContract {
    type Impl = FTBase;

    fn burn(e: &Env, from: &Address, amount: i128) {
        Self::assert_allowed(e, &[from]);
        Self::Impl::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Self::assert_allowed(e, &[from]);
        Self::Impl::burn_from(e, spender, from, amount);
    }
}

#[contract]
struct MockContract;

impl FungibleAllowList for MockContract {
    type Impl = AllowList;
}

#[test]
fn allow_user_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Check initial state
        assert!(!MockContract::allowed(&e, &user));

        // Allow user
        MockContract::allow_user(&e, &user, &user);

        // Verify user is allowed
        assert!(MockContract::allowed(&e, &user));
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
        MockContract::allow_user(&e, &user, &user);
        assert!(MockContract::allowed(&e, &user));
    });

    e.as_contract(&address, || {
        // Disallow user
        AllowList::disallow_user(&e, &user, &user);

        // Verify user is not allowed
        assert!(!MockContract::allowed(&e, &user));
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
        MockContract::allow_user(&e, &user1, &user1);
        MockContract::allow_user(&e, &user2, &user1);

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
fn allowlist_burn_override_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow user first
        MockContract::allow_user(&e, &user, &user);

        // Mint tokens to user
        MockContract::internal_mint(&e, &user, 100);

        // Burn tokens from user
        MockContract::burn(&e, &user, 50);

        // Verify balance
        assert_eq!(MockContract::balance(&e, &user), 50);
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
        MockContract::allow_user(&e, &user1, &user1);

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
#[should_panic(expected = "Error(Contract, #113)")]
fn transfer_with_sender_not_allowed_panics() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    e.as_contract(&address, || {
        // Allow only user2
        MockContract::allow_user(&e, &user2, &user1);

        // Mint tokens to user1
        MockContract::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 (not allowed) to user2
        MockContract::transfer(&e, &user1, &user2, 50);
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
        MockContract::allow_user(&e, &user1, &user1);

        // Mint tokens to user1
        MockContract::internal_mint(&e, &user1, 100);

        // Try to transfer tokens from user1 to user2 (not allowed)
        MockContract::transfer(&e, &user1, &user2, 50);
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
        MockContract::approve(&e, &user1, &user2, 50, 100);
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
        MockContract::internal_mint(&e, &user, 100);

        // Try to burn tokens from user (not allowed)
        MockContract::burn(&e, &user, 50);
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
        MockContract::internal_mint(&e, &user1, 100);

        // Allow user2 to burn tokens from user1
        MockContract::approve(&e, &user1, &user2, 50, 100);

        // Try to burn tokens from user1 by user2 (not allowed)
        MockContract::burn_from(&e, &user2, &user1, 50);
    });
}
