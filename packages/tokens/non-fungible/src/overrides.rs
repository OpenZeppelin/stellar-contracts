use soroban_sdk::{Address, Env};

use crate::TokenId;

/// Based on the Extension, some default behavior of [`crate::NonFungibleToken`]
/// might have to be overridden. This is a helper trait that allows us this
/// override mechanism.
pub trait ContractOverrides {
    fn owner_of(e: &Env, token_id: TokenId) -> Address;
    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId);
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId);
    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    );
}

/// Default marker type
pub struct Base;

impl ContractOverrides for Base {
    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        crate::owner_of(e, token_id)
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        crate::approve(e, &approver, &approved, token_id, live_until_ledger);
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        crate::transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        crate::transfer_from(e, &spender, &from, &to, token_id);
    }
}
