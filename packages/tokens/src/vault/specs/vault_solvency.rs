use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr::clog;
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use stellar_contract_utils::math::fixed_point::Rounding;

use crate::{
    fungible::FungibleToken,
    vault::{
        specs::{asset_token::AssetToken, vault::BasicVault},
        FungibleVault, Vault,
    },
};

use super::vault_invariants::safe_assumptions;

// todo
// total_assets = 0 iff total_supply = 0
// deposit back and forth favors the vault

// invariant: total_assets >= total_supply

// helpers

pub fn assume_pre_total_assets_geq_total_supply(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    cvlr_assume!(total_assets >= total_supply);
}

pub fn assert_post_total_assets_geq_total_supply(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    cvlr_assert!(total_assets >= total_supply);
}

#[rule]
// status:
pub fn after_transfer_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer(&e, from, to, amount);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_transfer_from_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer_from(&e, spender, from, to, amount);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_approve_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    BasicVault::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_deposit_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::deposit(&e, assets, receiver, from, operator);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_mint_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::mint(&e, shares, receiver, from, operator);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_withdraw_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::withdraw(&e, assets, receiver, owner, operator);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_redeem_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::redeem(&e, shares, receiver, owner, operator);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_set_asset_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let asset: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&asset));
    Vault::set_asset(&e, asset);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
// maybe this doesn't work? because it breaks the math?
pub fn after_set_decimals_offset_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let offset: u32 = nondet();
    clog!(offset);
    Vault::set_decimals_offset(&e, offset);
    assert_post_total_assets_geq_total_supply(&e);
}

// we can check also for the operations on the underlying token, so long as the current contract doesn't send tokens.

#[rule]
// status:
pub fn after_token_transfer_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let current_contract_address = e.current_contract_address();
    clog!(cvlr_soroban::Addr(&current_contract_address));
    cvlr_assume!(from != current_contract_address); // contract doesn't send its tokens
    AssetToken::transfer(&e, from, to, amount);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_token_transfer_from_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    let current_contract_address = e.current_contract_address();
    clog!(cvlr_soroban::Addr(&current_contract_address));
    cvlr_assume!(from != current_contract_address); // contract doesn't send its tokens
    AssetToken::transfer_from(&e, spender, from, to, amount);
    assert_post_total_assets_geq_total_supply(&e);
}

#[rule]
// status:
pub fn after_token_approve_total_assets_geq_total_supply(e: Env) {
    safe_assumptions(&e);
    assume_pre_total_assets_geq_total_supply(&e);
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    // contract can technically approve, that doesn't break solvency (yet)
    AssetToken::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_assets_geq_total_supply(&e);
}