use soroban_sdk::{contract, contractimpl, Address, Env, String};

use crate::fungible::{
    blocklist::{BlockList, FungibleBlockList},
    overrides::ContractOverrides,
    Base, FungibleToken,
};

pub struct FungibleBlockListContract;

impl FungibleToken for FungibleBlockListContract {
    type ContractType = BlockList;

    fn total_supply(e: &Env) -> i128 {
        BlockList::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        BlockList::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        BlockList::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        BlockList::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        BlockList::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        BlockList::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        BlockList::decimals(e)
    }

    fn name(e: &Env) -> String {
        BlockList::name(e)
    }

    fn symbol(e: &Env) -> String {
        BlockList::symbol(e)
    }
}

impl FungibleBlockList for FungibleBlockListContract {
    
    fn blocked(e: &Env, account: Address) -> bool {
        BlockList::blocked(e, &account)
    }
    
    fn block_user(e: &Env, user: Address, operator: Address) {
        BlockList::block_user(e, &user);
    }
    
    fn unblock_user(e: &Env, user: Address, operator: Address) {
        BlockList::unblock_user(e, &user);
    }
}
