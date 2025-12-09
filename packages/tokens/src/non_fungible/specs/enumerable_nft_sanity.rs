use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::non_fungible::{
    burnable::NonFungibleBurnable, enumerable::NonFungibleEnumerable,
    specs::enumerable_nft_contract::EnumerableNft, NonFungibleToken,
};

#[rule]
pub fn enumerable_nft_balance_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = EnumerableNft::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_owner_of_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = EnumerableNft::owner_of(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_transfer_sanity(e: Env) {
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    EnumerableNft::transfer(&e, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_transfer_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    EnumerableNft::transfer_from(&e, spender, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_approve_sanity(e: Env) {
    let approver: Address = nondet_address();
    let approved: Address = nondet_address();
    let token_id: u32 = nondet();
    let live_until_ledger: u32 = nondet();
    EnumerableNft::approve(&e, approver, approved, token_id, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_approve_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let live_until_ledger: u32 = nondet();
    EnumerableNft::approve_for_all(&e, owner, operator, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_get_approved_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = EnumerableNft::get_approved(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_is_approved_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let _ = EnumerableNft::is_approved_for_all(&e, owner, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_name_sanity(e: Env) {
    let _ = EnumerableNft::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_symbol_sanity(e: Env) {
    let _ = EnumerableNft::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_token_uri_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = EnumerableNft::token_uri(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_burn_sanity(e: Env) {
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    EnumerableNft::burn(&e, from, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_burn_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let token_id: u32 = nondet();
    EnumerableNft::burn_from(&e, spender, from, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_total_supply_sanity(e: Env) {
    let _ = EnumerableNft::total_supply(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_get_owner_token_id_sanity(e: Env) {
    let owner: Address = nondet_address();
    let index: u32 = nondet();
    let _ = EnumerableNft::get_owner_token_id(&e, owner, index);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_get_token_id_sanity(e: Env) {
    let index: u32 = nondet();
    let _ = EnumerableNft::get_token_id(&e, index);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_seq_mint_sanity(e: Env) {
    let to: Address = nondet_address();
    let _ = EnumerableNft::seq_mint(&e, to);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enumerable_nft_nonseq_mint_sanity(e: Env) {
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    EnumerableNft::nonseq_mint(&e, to, token_id);
    cvlr_satisfy!(true);
}
