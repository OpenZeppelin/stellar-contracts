use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    burnable::NonFungibleBurnable,
    consecutive::{Consecutive, NonFungibleConsecutive},
    ContractOverrides, NonFungibleToken,
};

pub struct ConsecutiveNft;

impl NonFungibleToken for ConsecutiveNft {
    type ContractType = Consecutive;

    fn balance(e: &Env, account: Address) -> u32 {
        Consecutive::balance(e, &account)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        Consecutive::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Consecutive::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        Consecutive::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Consecutive::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Consecutive::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        Consecutive::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Consecutive::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        Consecutive::name(e)
    }

    fn symbol(e: &Env) -> soroban_sdk::String {
        Consecutive::symbol(e)
    }

    fn token_uri(e: &Env, token_id: u32) -> soroban_sdk::String {
        Consecutive::token_uri(e, token_id)
    }
}

impl NonFungibleBurnable for ConsecutiveNft {
    fn burn(e: &Env, from: Address, token_id: u32) {
        Consecutive::burn(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        Consecutive::burn_from(e, &spender, &from, token_id);
    }
}
// TODO: This is just a basic call to batch_mint. May need additional setup
// depending on rule.
impl ConsecutiveNft {
    pub fn batch_mint(e: &Env, to: Address, amount: u32) -> u32 {
        Consecutive::batch_mint(e, &to, amount)
    }
}

impl NonFungibleConsecutive for ConsecutiveNft {}
