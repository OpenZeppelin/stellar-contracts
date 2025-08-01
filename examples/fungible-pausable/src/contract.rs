//! Fungible Pausable Example Contract.

//! This contract showcases how to integrate various OpenZeppelin modules to
//! build a fully SEP-41-compliant fungible token. It includes essential
//! features such as an emergency stop mechanism and controlled token minting by
//! the owner.
//!
//! To meet SEP-41 compliance, the contract must implement both
//! [`stellar_fungible::fungible::FungibleToken`] and
//! [`stellar_fungible::burnable::FungibleBurnable`].

use soroban_sdk::{contract, contractimpl, contracttrait, Address, Env, String};
use stellar_access::Ownable;
use stellar_contract_utils::Pausable;
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::{impl_token_interface, FungibleBurnable, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        Self::set_metadata(e, 18, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
        Self::set_owner(e, &owner);
        Self::internal_mint(e, &owner, initial_supply);
    }

    #[when_not_paused]
    pub fn mint(e: &Env, account: Address, amount: i128) {
        Self::enforce_owner_auth(e);
        Self::internal_mint(e, &account, amount);
    }
}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contracttrait]
impl FungibleToken for ExampleContract {
    #[when_not_paused]
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::Impl::transfer(e, from, to, amount);
    }

    #[when_not_paused]
    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }
}

#[contracttrait]
impl FungibleBurnable for ExampleContract {
    #[when_not_paused]
    fn burn(e: &Env, from: &Address, amount: i128) {
        Self::Impl::burn(e, from, amount)
    }

    #[when_not_paused]
    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Self::Impl::burn_from(e, spender, from, amount)
    }
}

#[contracttrait]
impl Pausable for ExampleContract {
    #[only_owner]
    fn pause(e: &Env, caller: &Address) {
        Self::Impl::pause(e, caller);
    }

    #[only_owner]
    fn unpause(e: &Env, caller: &Address) {
        Self::Impl::unpause(e, caller);
    }
}

// NOTE: if your contract implements `FungibleToken` and `FungibleBurnable`,
// and you also want your contract to implement
// `soroban_sdk::token::TokenInterface`, you can use the `impl_token_interface!`
// macro to generate the boilerplate implementation.
impl_token_interface!(ExampleContract);
