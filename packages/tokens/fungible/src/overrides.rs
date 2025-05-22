use soroban_sdk::{Address, Env, String};

/// Based on the extension, some default behavior of [`crate::FungibleToken`]
/// might have to be overridden. This is a helper trait that allows us this
/// override mechanism that favors the DevX.
///
/// One can also override the `FungibleToken` trait directly, but the reason
/// we have another trait for the same methods, is to provide the default
/// implementations in an easier way for the end developer.
///
/// The way to provide different default implementations for different
/// extensions is by implementing the trait for different types (unit structs).
/// The problem is, `FungibleToken` trait has to be implemented for the smart
/// contract (which is another struct) by the end-developer. So, we need a level
/// of abstraction by introducing an associated type, which will grant
/// `FungibleToken` trait the ability to switch between different default
/// implementations by calling the methods on this associated type. And for
/// this, we need another trait, which this associated type will implement.
///
/// By introducing this abstraction, we allow the end-developer to implement
/// every method of the `FungibleToken` trait using
/// `Self::ContractType::{function_name}`, which will in turn use either the
/// overridden or the base variant according to the extension, provided by the
/// `ContractOverrides` trait implementation for the respective `ContractType`.
pub trait ContractOverrides {
    fn total_supply(e: &Env) -> i128 {
        Base::total_supply(e)
    }

    fn balance(e: &Env, account: &Address) -> i128 {
        Base::balance(e, account)
    }

    fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
        Base::allowance(e, owner, spender)
    }

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Base::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Base::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Base::decimals(e)
    }

    fn name(e: &Env) -> String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol(e)
    }
}

/// Default marker type
pub struct Base;

// No override required for the `Base` contract type.
impl ContractOverrides for Base {
    // Default implementation is already provided by the trait
}

// Helper functions for the Base implementation
impl Base {
    pub fn total_supply(e: &Env) -> i128 {
        crate::total_supply(e)
    }

    pub fn balance(e: &Env, account: &Address) -> i128 {
        crate::balance(e, account)
    }

    pub fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
        crate::allowance(e, owner, spender)
    }

    pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        crate::transfer(e, from, to, amount);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        crate::transfer_from(e, spender, from, to, amount);
    }

    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        crate::approve(e, owner, spender, amount, live_until_ledger);
    }

    pub fn decimals(e: &Env) -> u32 {
        crate::metadata::decimals(e)
    }

    pub fn name(e: &Env) -> String {
        crate::metadata::name(e)
    }

    pub fn symbol(e: &Env) -> String {
        crate::metadata::symbol(e)
    }
}
