use soroban_sdk::{contract, contractimpl, Address, Env, String};

use crate::fungible::{
    Base, FungibleToken, allowlist::{AllowList, FungibleAllowList}, capped::{check_cap, set_cap}, overrides::ContractOverrides
};
use crate::fungible::burnable::FungibleBurnable;

// NOTE: This implements the `FungibleBurnable` trait on top of AllowList.
// NOTE: This also uses the capped functionality to make sure they get verified.
// Please change or split into separate files as needed.

pub struct FungibleAllowListContract;

impl FungibleAllowListContract {
        pub fn __constructor(e: &Env, cap: i128) {
        set_cap(e, cap);
    }

    pub fn mint(e: &Env, account: Address, amount: i128) {
        check_cap (e, amount);
        Base::mint(e, &account, amount);
    }
}

impl FungibleToken for FungibleAllowListContract {
    type ContractType = AllowList;

    fn total_supply(e: &Env) -> i128 {
        AllowList::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        AllowList::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        AllowList::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        AllowList::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        AllowList::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        AllowList::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        AllowList::decimals(e)
    }

    fn name(e: &Env) -> String {
        AllowList::name(e)
    }

    fn symbol(e: &Env) -> String {
        AllowList::symbol(e)
    }
}

impl FungibleAllowList for FungibleAllowListContract {
    fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }

    fn allow_user(e: &Env, user: Address, operator: Address) {
        AllowList::allow_user(e, &user);
    }

    fn disallow_user(e: &Env, user: Address, operator: Address) {
        AllowList::disallow_user(e, &user)
    }
}

impl FungibleBurnable for FungibleAllowListContract {
    fn burn(e: &Env, from: Address, amount: i128) {
        Base::burn(e, &from, amount);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        Base::burn_from(e, &spender, &from, amount);
    }
}