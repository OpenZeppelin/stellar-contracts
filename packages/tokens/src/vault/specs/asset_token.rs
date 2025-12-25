use soroban_sdk::{contract, contractimpl, Address, Env, String};

use crate::fungible::{Base, FungibleToken};

pub struct AssetToken<'a> {
    pub asset: &'a Address,
}

impl<'a> AssetToken<'a> {
    pub fn __constructor(e: &Env, addr: &'a Address) -> AssetToken<'a> {
        return AssetToken { asset: addr };
    }
}

impl<'a> FungibleToken for AssetToken<'a> {
    type ContractType = Base;

    fn total_supply(e: &Env) -> i128 {
        Base::total_supply_munged(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Base::balance_munged(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Base::allowance_munged(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        Base::transfer_munged(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Base::transfer_from_munged(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Base::approve_munged(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Base::decimals_munged(e)
    }

    fn name(e: &Env) -> String {
        Base::name_munged(e)
    }

    fn symbol(e: &Env) -> String {
        Base::symbol_munged(e)
    }
}
