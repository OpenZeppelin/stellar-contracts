use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};

use crate::non_fungible::specs::burnable_nft_contract::BurnableNft;
use crate::non_fungible::{NonFungibleToken, burnable::NonFungibleBurnable};

#[rule]
pub fn burnable_nft_balance_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = BurnableNft::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_owner_of_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = BurnableNft::owner_of(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_transfer_sanity(e: Env) {
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    BurnableNft::transfer(&e, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_transfer_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    BurnableNft::transfer_from(&e, spender, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_approve_sanity(e: Env) {
    let approver: Address = nondet_address();
    let approved: Address = nondet_address();
    let token_id: u32 = nondet();
    let live_until_ledger: u32 = nondet();
    BurnableNft::approve(&e, approver, approved, token_id, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_approve_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let live_until_ledger: u32 = nondet();
    BurnableNft::approve_for_all(&e, owner, operator, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_get_approved_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = BurnableNft::get_approved(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_is_approved_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let _ = BurnableNft::is_approved_for_all(&e, owner, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_name_sanity(e: Env) {
    let _ = BurnableNft::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_symbol_sanity(e: Env) {
    let _ = BurnableNft::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_token_uri_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = BurnableNft::token_uri(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_burn_sanity(e: Env) {
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    BurnableNft::burn(&e, from, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn burnable_nft_burn_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    BurnableNft::burn_from(&e, spender, from, token_id);
    cvlr_satisfy!(true);
}
