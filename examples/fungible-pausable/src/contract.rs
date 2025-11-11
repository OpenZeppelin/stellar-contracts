//! Fungible Pausable Example Contract.

//! This contract showcases how to integrate various OpenZeppelin modules to
//! build a fully SEP-41-compliant fungible token. It includes essential
//! features such as an emergency stop mechanism and controlled token minting by
//! the owner.
//!
//! To meet SEP-41 compliance, the contract must implement both
//! [`stellar_fungible::fungible::FungibleToken`] and
//! [`stellar_fungible::burnable::FungibleBurnable`].

use soroban_sdk::{
    Address, Env, MuxedAddress, String, Symbol, contract, contracterror, contractimpl, panic_with_error, symbol_short
};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::when_not_paused;
use stellar_tokens::fungible::{burnable::FungibleBurnable, Base, FungibleToken};

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
        Base::set_metadata(e, 18, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
        Base::mint(e, &owner, initial_supply);
        e.storage().instance().set(&OWNER, &owner);
    }

    #[when_not_paused]
    pub fn mint(e: &Env, account: Address, amount: i128) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        owner.require_auth();

        Base::mint(e, &account, amount);
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
        caller.require_auth();
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::pause(e);
    }

    fn unpause(e: &Env, caller: Address) {
        // When `ownable` module is available,
        // the following checks should be equivalent to:
        // `ownable::only_owner(&e);`
        caller.require_auth();
        let owner: Address = e.storage().instance().get(&OWNER).expect("owner should be set");
        if owner != caller {
            panic_with_error!(e, ExampleContractError::Unauthorized);
        }

        pausable::unpause(e);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Base;

    fn total_supply(e: &Env) -> i128 {
        Self::ContractType::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Self::ContractType::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Self::ContractType::allowance(e, &owner, &spender)
    }

    #[when_not_paused]
    fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
        Self::ContractType::transfer(e, &from, &to, amount);
    }

    #[when_not_paused]
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Self::ContractType::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Self::ContractType::decimals(e)
    }

    fn name(e: &Env) -> String {
        Self::ContractType::name(e)
    }

    fn symbol(e: &Env) -> String {
        Self::ContractType::symbol(e)
    }
}

#[contractimpl]
impl FungibleBurnable for ExampleContract {
    #[when_not_paused]
    fn burn(e: &Env, from: Address, amount: i128) {
        Self::ContractType::burn(e, &from, amount)
    }

    #[when_not_paused]
    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        Self::ContractType::burn_from(e, &spender, &from, amount)
    }
}
