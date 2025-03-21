mod storage;
pub use self::storage::{increment_token_id, next_token_id};

use soroban_sdk::Env;

pub trait NonFungibleSequential {
    fn next_token_id(e: &Env) -> u32;

    fn increment_token_id(e: &Env, amount: u32) -> u32;
}
