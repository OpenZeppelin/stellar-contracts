use soroban_sdk::{Address, Env, String};

use crate::{consecutive::storage, ContractOverrides, TokenId};

pub struct Consecutive;

impl ContractOverrides for Consecutive {
    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        self::storage::consecutive_owner_of(e, token_id)
    }

    fn token_uri(e: &Env, token_id: TokenId) -> String {
        self::storage::consecutive_token_uri(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        self::storage::consecutive_transfer(e, &from, &to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        self::storage::consecutive_transfer_from(e, &spender, &from, &to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        self::storage::consecutive_approve(e, &approver, &approved, token_id, live_until_ledger);
    }
}
