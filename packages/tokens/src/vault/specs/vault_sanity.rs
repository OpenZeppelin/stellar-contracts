use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};

use stellar_contract_utils::math::fixed_point::Rounding;

use crate::vault::{FungibleVault, Vault};
use crate::vault::specs::vault::BasicVault;

// Note: we are currently not depending on `query_asset`. Could be something to fix

#[rule]
pub fn vault_query_asset_sanity(e: Env) {
	let _ = BasicVault::query_asset(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_total_assets_sanity(e: Env) {
	let _ = BasicVault::total_assets(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_convert_to_shares_sanity(e: Env) {
	let assets: i128 = nondet();
	let _ = BasicVault::convert_to_shares(&e, assets);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_convert_to_assets_sanity(e: Env) {
	let shares: i128 = nondet();
	let _ = BasicVault::convert_to_assets(&e, shares);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_max_deposit_sanity(e: Env) {
	let receiver: Address = nondet_address();
	let _ = BasicVault::max_deposit(&e, receiver);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_preview_deposit_sanity(e: Env) {
	let assets: i128 = nondet();
	let _ = BasicVault::preview_deposit(&e, assets);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_max_mint_sanity(e: Env) {
	let receiver: Address = nondet_address();
	let _ = BasicVault::max_mint(&e, receiver);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_preview_mint_sanity(e: Env) {
	let shares: i128 = nondet();
	let _ = BasicVault::preview_mint(&e, shares);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_max_withdraw_sanity(e: Env) {
	let owner: Address = nondet_address();
	let _ = BasicVault::max_withdraw(&e, owner);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_preview_withdraw_sanity(e: Env) {
	let assets: i128 = nondet();
	let _ = BasicVault::preview_withdraw(&e, assets);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_max_redeem_sanity(e: Env) {
	let owner: Address = nondet_address();
	let _ = BasicVault::max_redeem(&e, owner);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_preview_redeem_sanity(e: Env) {
	let shares: i128 = nondet();
	let _ = BasicVault::preview_redeem(&e, shares);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_deposit_sanity(e: Env) {
	let assets: i128 = nondet();
	let receiver: Address = nondet_address();
	let from: Address = nondet_address();
	let operator: Address = nondet_address();
	let _ = BasicVault::deposit(&e, assets, receiver, from, operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_mint_sanity(e: Env) {
	let shares: i128 = nondet();
	let receiver: Address = nondet_address();
	let from: Address = nondet_address();
	let operator: Address = nondet_address();
	let _ = BasicVault::mint(&e, shares, receiver, from, operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_withdraw_sanity(e: Env) {
	let assets: i128 = nondet();
	let receiver: Address = nondet_address();
	let owner: Address = nondet_address();
	let operator: Address = nondet_address();
	let _ = BasicVault::withdraw(&e, assets, receiver, owner, operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_redeem_sanity(e: Env) {
	let shares: i128 = nondet();
	let receiver: Address = nondet_address();
	let owner: Address = nondet_address();
	let operator: Address = nondet_address();
	let _ = BasicVault::redeem(&e, shares, receiver, owner, operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_decimals_sanity(e: Env) {
	let _ = Vault::decimals(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_set_assset_sanity(e: Env) {
	let asset = nondet_address();
    Vault::set_asset(&e, asset);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_set_decimals_offset_sanity(e: Env) {
    let offset: u32 = nondet();
	Vault::set_decimals_offset(&e, offset);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_deposit_internal_sanity(e: Env) {
	let receiver: Address = nondet_address();
	let assets: i128 = nondet();
	let shares: i128 = nondet();
	let from: Address = nondet_address();
	let operator: Address = nondet_address();
	Vault::deposit_internal(&e, &receiver, assets, shares, &from, &operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_withdraw_internal_sanity(e: Env) {
	let receiver: Address = nondet_address();
	let owner: Address = nondet_address();
	let assets: i128 = nondet();
	let shares: i128 = nondet();
	let operator: Address = nondet_address();
	Vault::withdraw_internal(&e, &receiver, &owner, assets, shares, &operator);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_get_decimals_offset_sanity(e: Env) {
	let _ = Vault::get_decimals_offset(&e);
	cvlr_satisfy!(true);
}

#[rule]
pub fn vault_get_underlying_asset_decimals_sanity(e: Env) {
	let _ = Vault::get_underlying_asset_decimals(&e);
	cvlr_satisfy!(true);
}

