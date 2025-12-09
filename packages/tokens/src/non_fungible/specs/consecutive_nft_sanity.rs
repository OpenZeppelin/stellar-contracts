use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    burnable::NonFungibleBurnable, consecutive::NonFungibleConsecutive,
    specs::consecutive_nft_contract::ConsecutiveNft, NonFungibleToken,
};

#[rule]
pub fn consecutive_nft_balance_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = ConsecutiveNft::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_owner_of_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = ConsecutiveNft::owner_of(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_transfer_sanity(e: Env) {
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    ConsecutiveNft::transfer(&e, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_transfer_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    ConsecutiveNft::transfer_from(&e, spender, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_approve_sanity(e: Env) {
    let approver: Address = nondet_address();
    let approved: Address = nondet_address();
    let token_id: u32 = nondet();
    let live_until_ledger: u32 = nondet();
    ConsecutiveNft::approve(&e, approver, approved, token_id, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_approve_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let live_until_ledger: u32 = nondet();
    ConsecutiveNft::approve_for_all(&e, owner, operator, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_get_approved_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = ConsecutiveNft::get_approved(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_is_approved_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let _ = ConsecutiveNft::is_approved_for_all(&e, owner, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_name_sanity(e: Env) {
    let _ = ConsecutiveNft::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_symbol_sanity(e: Env) {
    let _ = ConsecutiveNft::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_token_uri_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = ConsecutiveNft::token_uri(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_burn_sanity(e: Env) {
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    ConsecutiveNft::burn(&e, from, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_burn_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    ConsecutiveNft::burn_from(&e, spender, from, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn consecutive_nft_batch_mint_sanity(e: Env) {
    let to: Address = nondet_address();
    let amount: u32 = nondet();
    let _ = ConsecutiveNft::batch_mint(&e, to, amount);
    cvlr_satisfy!(true);
}
