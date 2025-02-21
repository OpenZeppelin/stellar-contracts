//! Fungible Pausable Example Contract.

//! This contract showcases how to integrate various OpenZeppelin modules to
//! build a fully SEP-41-compliant fungible token. It includes essential
//! features such as an emergency stop mechanism and controlled token minting by
//! the owner.
//!
//! To meet SEP-41 compliance, the contract must implement both
//! [`openzeppelin_fungible_token::fungible::FungibleToken`] and
//! [`openzeppelin_fungible_token::burnable::FungibleBurnable`].

use openzeppelin_fungible_token::{
    self as fungible, burnable::FungibleBurnable, mintable::FungibleMintable, FungibleToken,
};
use openzeppelin_pausable::{self as pausable, Pausable};
use openzeppelin_pausable_macros::when_not_paused;
use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, String,
    Symbol,
};

pub const OWNER: Symbol = symbol_short!("OWNER");

#[contract]
pub struct ExampleContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        fungible::metadata::set_metadata(
            e,
            18,
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
        fungible::mintable::mint(e, &owner, initial_supply);
        e.storage().instance().set(&OWNER, &owner);
    }
}

#[contractimpl]
impl Pausable for ExampleContract {
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    fn pause(e: &Env, caller: Address) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::pause(e, &caller);
    }

    fn unpause(e: &Env, caller: Address) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::unpause(e, &caller);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    fn total_supply(e: &Env) -> i128 {
        fungible::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        fungible::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        fungible::allowance(e, &owner, &spender)
    }

    #[when_not_paused]
    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        fungible::transfer(e, &from, &to, amount);
    }

    #[when_not_paused]
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        fungible::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        fungible::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        fungible::metadata::decimals(e)
    }

    fn name(e: &Env) -> String {
        fungible::metadata::name(e)
    }

    fn symbol(e: &Env) -> String {
        fungible::metadata::symbol(e)
    }
}

#[contractimpl]
impl FungibleBurnable for ExampleContract {
    #[when_not_paused]
    fn burn(e: &Env, from: Address, amount: i128) {
        fungible::burnable::burn(e, &from, amount)
    }

    #[when_not_paused]
    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        fungible::burnable::burn_from(e, &spender, &from, amount)
    }
}

#[contractimpl]
impl FungibleMintable for ExampleContract {
    #[when_not_paused]
    fn mint(e: &Env, account: Address, amount: i128) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        owner.require_auth();

        fungible::mintable::mint(e, &account, amount);
    }
}
