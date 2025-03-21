#![allow(unused_variables)]
pub mod storage;
use soroban_sdk::{Address, Env, Symbol};

mod test;

pub trait INonFungibleBase {
    fn transfer(e: &Env, from: Address, to: Address, token_id: u32);

    fn owner_of(e: &Env, token_id: u32) -> Address;

    fn increase_balance(e: &Env, to: Address, amount: u32);

    fn decrease_balance(e: &Env, from: Address, amount: u32);
}

pub trait IMintable: INonFungibleBase {
    fn mint(e: &Env, to: Address, token_id: u32) -> u32;
}

pub trait IBurnable: INonFungibleBase {
    fn burn(e: &Env, from: Address, token_id: u32);

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32);
}

pub trait ISequential {
    fn next_token_id(e: &Env) -> u32;

    fn increment_token_id(e: &Env) -> u32;

    fn increment_token_id_by(e: &Env, amount: u32) -> u32;
}

pub trait INonFungibleConsecutive: INonFungibleBase + ISequential + IMintable + IBurnable {
    fn batch_mint(e: &Env, to: Address, amount: u32);
}

// ################## EVENTS ##################

/// Emits an event indicating a mint of a token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new token.
/// * `token_id` - Token id as a number.
///
/// # Events
///
/// * topics - `["consecutive_mint", to: Address]`
/// * data - `[from_token_id: u32, to_token_id: u32]`
pub fn emit_consecutive_mint(e: &Env, to: &Address, from_token_id: u32, to_token_id: u32) {
    let topics = (Symbol::new(e, "consecutive_mint"), to);
    e.events().publish(topics, (from_token_id, to_token_id))
}
