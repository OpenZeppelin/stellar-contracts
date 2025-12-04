use soroban_sdk::{contract, contractimpl, Address, Env, String};

use crate::fungible::{extensions::capped::{check_cap, query_cap, set_cap}, Base, FungibleToken};

#[contract]
pub struct CappedTokenContract;

#[contractimpl]
impl CappedTokenContract {
    pub fn __constructor(e: &Env, cap: i128) {
        set_cap(e, cap);
    }

    pub fn mint(e: &Env, account: Address, amount: i128) {
        check_cap(e, amount);
        Base::mint(e, &account, amount);
    }

    pub fn get_cap(e: &Env) -> i128 {
        query_cap(e)
    }
}

#[contractimpl]
impl FungibleToken for CappedTokenContract {
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

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer(e, &from, &to, amount);
    }

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
