use soroban_sdk::{Address, Env};

use crate::{ContractOverrides, TokenId};

pub struct Enumerable;

impl ContractOverrides for Enumerable {
    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        crate::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        crate::extensions::enumerable::storage::enumerable_transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        crate::extensions::enumerable::storage::enumerable_transfer_from(
            e, &spender, &from, &to, token_id,
        );
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
}
