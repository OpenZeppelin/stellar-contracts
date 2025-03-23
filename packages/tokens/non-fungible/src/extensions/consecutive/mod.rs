pub mod storage;
use soroban_sdk::{Address, Env, Symbol};

use crate::NonFungibleToken;

mod test;

pub trait NonFungibleConsecutive: NonFungibleToken {}

// ################## EVENTS ##################

/// Emits an event indicating a mint of a batch of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new token.
/// * `from_token_id` - First token id in the batch.
/// * `to_token_id` - Last token id of the batch.
///
/// # Events
///
/// * topics - `["consecutive_mint", to: Address]`
/// * data - `[from_token_id: u32, to_token_id: u32]`
pub fn emit_consecutive_mint(e: &Env, to: &Address, from_token_id: u32, to_token_id: u32) {
    let topics = (Symbol::new(e, "consecutive_mint"), to);
    e.events().publish(topics, (from_token_id, to_token_id))
}
