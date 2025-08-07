//! Fungible Pausable Example Contract.

//! This contract showcases how to integrate various OpenZeppelin modules to
//! build a fully SEP-41-compliant fungible token. It includes essential
//! features such as an emergency stop mechanism and controlled token minting by
//! the owner.
//!
//! This contract replicates the functionality of the contract in
//! "examples/fungible-pausable", offering the same features. The key difference
//! lies in how SEP-41 compliance is achieved. The contract in "contract.rs"
//! accomplishes this by implementing
//! [`stellar_tokens::fungible::FungibleToken`] and
//! [`stellar_tokens::fungible_burnable::FungibleBurnable`], whereas this
//! version directly implements [`soroban_sdk::token::TokenInterface`].
//!
//! Ultimately, it is up to the user to choose their preferred approach to
//! creating a SEP-41 token. We suggest the approach in
//! "examples/fungible-pausable" for better organization of the code,
//! consistency and ease of inspection/debugging.

use soroban_sdk::{contract, contractimpl, token::TokenInterface, Address, Env, String};
use stellar_access::{Ownable, Owner};
use stellar_contract_utils::{Pausable, PausableDefault};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::{FTBase, FungibleBurnable, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        Self::set_owner(e, &owner);
        FTBase::set_metadata(e, 18, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
        FTBase::internal_mint(e, &owner, initial_supply);
    }

    /// `TokenInterface` doesn't require implementing `total_supply()` because
    /// of the need for backwards compatibility with Stellar classic assets.
    pub fn total_supply(e: &Env) -> i128 {
        FTBase::total_supply(e)
    }

    #[when_not_paused]
    #[only_owner]
    pub fn mint(e: &Env, account: Address, amount: i128) {
        FTBase::internal_mint(e, &account, amount);
    }
}

#[contractimpl]
impl Ownable for ExampleContract {
    type Impl = Owner;
}

pub struct OwnableExt<T: Ownable, N>(T, N);

impl<T: Ownable, N: Pausable> Pausable for OwnableExt<T, N> {
    type Impl = N;

    fn pause(e: &Env, caller: &Address) {
        T::only_owner(e);
        Self::Impl::pause(e, caller);
    }

    fn unpause(e: &Env, caller: &Address) {
        T::only_owner(e);
        Self::Impl::unpause(e, caller);
    }
}

#[contractimpl]
impl Pausable for ExampleContract {
    type Impl = OwnableExt<Self, PausableDefault>;
}

#[contractimpl]
impl TokenInterface for ExampleContract {
    fn balance(e: Env, account: Address) -> i128 {
        FTBase::balance(&e, &account)
    }

    fn allowance(e: Env, owner: Address, spender: Address) -> i128 {
        FTBase::allowance(&e, &owner, &spender)
    }

    #[when_not_paused]
    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        FTBase::transfer(&e, &from, &to, amount);
    }

    #[when_not_paused]
    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        FTBase::transfer_from(&e, &spender, &from, &to, amount);
    }

    fn approve(e: Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        FTBase::approve(&e, &owner, &spender, amount, live_until_ledger);
    }

    #[when_not_paused]
    fn burn(e: Env, from: Address, amount: i128) {
        FTBase::burn(&e, &from, amount)
    }

    #[when_not_paused]
    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        FTBase::burn_from(&e, &spender, &from, amount)
    }

    fn decimals(e: Env) -> u32 {
        FTBase::decimals(&e)
    }

    fn name(e: Env) -> String {
        FTBase::name(&e)
    }

    fn symbol(e: Env) -> String {
        FTBase::symbol(&e)
    }
}
