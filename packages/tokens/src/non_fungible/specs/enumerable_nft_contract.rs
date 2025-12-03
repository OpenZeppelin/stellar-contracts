use soroban_sdk::{Address, Env, xdr::Enum};

use crate::{non_fungible::Base, non_fungible::{ContractOverrides, NonFungibleToken, burnable::NonFungibleBurnable, enumerable::{Enumerable, NonFungibleEnumerable}}};

pub struct EnumerableNft;

impl NonFungibleToken for EnumerableNft {
    type ContractType = Enumerable;
    
    fn balance(e: &Env, account: Address) -> u32 {
        Enumerable::balance(e, &account)
    }
    
    fn owner_of(e: &Env, token_id: u32) -> Address {
        Enumerable::owner_of(e, token_id)
    }
    
    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Enumerable::transfer(e, &from, &to, token_id);
    }
    
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        Enumerable::transfer_from(e, &spender, &from, &to, token_id);
    }
    
    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Enumerable::approve(e, &approver, &approved, token_id, live_until_ledger);
    }
    
    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Enumerable::approve_for_all(e, &owner, &operator, live_until_ledger);
    }
    
    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        Enumerable::get_approved(e, token_id)
    }
    
    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Enumerable::is_approved_for_all(e, &owner, &operator)
    }
    
    fn name(e: &Env) -> soroban_sdk::String {
        Enumerable::name(e)
    }
    
    fn symbol(e: &Env) -> soroban_sdk::String {
        Enumerable::symbol(e)
    }
    
    fn token_uri(e: &Env, token_id: u32) -> soroban_sdk::String {
        Enumerable::token_uri(e, token_id)
    }
}

impl NonFungibleBurnable for EnumerableNft {
    fn burn(e: &Env, from: Address, token_id: u32) {
        Enumerable::burn(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        Enumerable::burn_from(e, &spender, &from, token_id);
    }
}

impl NonFungibleEnumerable for EnumerableNft {
    fn total_supply(e: &Env) -> u32 {
        Enumerable::total_supply(e)
    }

    fn get_owner_token_id(e: &Env, owner: Address, index: u32) -> u32 {
        Enumerable::get_owner_token_id(e, &owner, index)
    }

    fn get_token_id(e: &Env, index: u32) -> u32 {
        Enumerable::get_token_id(e, index)
    }
}

// TODO: Just a basic calls to `sequential_mint` and `non_sequential_mint`. May need additional setup depending on rule.
impl EnumerableNft {
    pub fn seq_mint(e: &Env, to: Address) -> u32 {
        Enumerable::sequential_mint(e, &to)
    }

    pub fn nonseq_mint(e: &Env, to: Address, token_id: u32) {
        Enumerable::non_sequential_mint(e, &to, token_id)
    }
}
