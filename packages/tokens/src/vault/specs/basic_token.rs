use soroban_sdk::{contract, contractimpl, Address, Env, String};

use crate::fungible::{Base, FungibleToken};


// NOTE: This implements the `FungibleBurnable` trait on top of AllowList.
// NOTE: This also uses the capped functionality to make sure they get verified.
// Please change or split into separate files as needed.

pub struct BasicToken<'a> {
    pub asset: &'a Address
}

impl<'a> BasicToken<'a> {
    pub fn __constructor (e: &Env, addr: &'a Address) -> BasicToken<'a> {
        return BasicToken { asset: addr };
    }
}

impl<'a> FungibleToken for BasicToken<'a> {
    type ContractType = Base;

    fn total_supply(e: &Env) -> i128 {
        Base::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Base::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Base::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        Base::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Base::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Base::approve(e, &owner, &spender, amount, live_until_ledger);
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