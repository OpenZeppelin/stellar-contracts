use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};

use crate::non_fungible::specs::royalties_nft_contract::RoyaltiesNft;
use crate::non_fungible::{NonFungibleToken, royalties::NonFungibleRoyalties};

#[rule]
pub fn royalties_nft_balance_sanity(e: Env) {
    let account: Address = nondet_address();
    let _ = RoyaltiesNft::balance(&e, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_owner_of_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = RoyaltiesNft::owner_of(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_transfer_sanity(e: Env) {
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    RoyaltiesNft::transfer(&e, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_transfer_from_sanity(e: Env) {
    let spender: Address = nondet_address();
    let from: Address = nondet_address();
    let to: Address = nondet_address();
    let token_id: u32 = nondet();
    RoyaltiesNft::transfer_from(&e, spender, from, to, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_approve_sanity(e: Env) {
    let approver: Address = nondet_address();
    let approved: Address = nondet_address();
    let token_id: u32 = nondet();
    let live_until_ledger: u32 = nondet();
    RoyaltiesNft::approve(&e, approver, approved, token_id, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_approve_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let live_until_ledger: u32 = nondet();
    RoyaltiesNft::approve_for_all(&e, owner, operator, live_until_ledger);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_get_approved_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = RoyaltiesNft::get_approved(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_is_approved_for_all_sanity(e: Env) {
    let owner: Address = nondet_address();
    let operator: Address = nondet_address();
    let _ = RoyaltiesNft::is_approved_for_all(&e, owner, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_name_sanity(e: Env) {
    let _ = RoyaltiesNft::name(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_symbol_sanity(e: Env) {
    let _ = RoyaltiesNft::symbol(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_token_uri_sanity(e: Env) {
    let token_id: u32 = nondet();
    let _ = RoyaltiesNft::token_uri(&e, token_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_set_default_royalty_sanity(e: Env) {
    let receiver: Address = nondet_address();
    let basis_points: u32 = nondet();
    let operator: Address = nondet_address();
    RoyaltiesNft::set_default_royalty(&e, receiver, basis_points, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_set_token_royalty_sanity(e: Env) {
    let token_id: u32 = nondet();
    let receiver: Address = nondet_address();
    let basis_points: u32 = nondet();
    let operator: Address = nondet_address();
    RoyaltiesNft::set_token_royalty(&e, token_id, receiver, basis_points, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_remove_token_royalty_sanity(e: Env) {
    let token_id: u32 = nondet();
    let operator: Address = nondet_address();
    RoyaltiesNft::remove_token_royalty(&e, token_id, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn royalties_nft_royalty_info_sanity(e: Env) {
    let token_id: u32 = nondet();
    let sale_price: i128 = nondet();
    let _ = RoyaltiesNft::royalty_info(&e, token_id, sale_price);
    cvlr_satisfy!(true);
}
