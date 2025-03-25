mod storage;
use soroban_sdk::Env;

pub use self::storage::{increment_token_id, next_token_id};
use crate::TokenId;

pub trait NonFungibleSequential {
    fn next_token_id(e: &Env) -> TokenId;

    fn increment_token_id(e: &Env, amount: TokenId) -> TokenId;
}
