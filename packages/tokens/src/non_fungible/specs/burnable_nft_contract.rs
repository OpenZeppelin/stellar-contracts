use soroban_sdk::{Address, Env};

use crate::non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

pub struct BurnableNft;

impl NonFungibleToken for BurnableNft {
    type ContractType = Base;

    fn balance(e: &Env, account: Address) -> u32 {
        Base::balance(e, &account)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        Base::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Base::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        Base::transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Base::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        Base::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        Base::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        Base::is_approved_for_all(e, &owner, &operator)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        Base::name(e)
    }

    fn symbol(e: &Env) -> soroban_sdk::String {
        Base::symbol(e)
    }

    fn token_uri(e: &Env, token_id: u32) -> soroban_sdk::String {
        Base::token_uri(e, token_id)
    }
}

impl NonFungibleBurnable for BurnableNft {
    fn burn(e: &Env, from: Address, token_id: u32) {
        Base::burn(e, &from, token_id);
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        Base::burn_from(e, &spender, &from, token_id);
    }
}
