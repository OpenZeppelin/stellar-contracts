use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use cvlr::clog;
use stellar_contract_utils::math::fixed_point::Rounding;

use crate::{
    fungible::FungibleToken,
    vault::{
        specs::{asset_token::AssetToken, vault::BasicVault},
        FungibleVault, Vault,
    },
};
use super::vault_invariants::safe_assumptions;
use super::vault_solvency::assume_pre_solvency;

pub fn useful_clogs(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    let decimals_offset = Vault::get_decimals_offset(e);
    clog!(decimals_offset);
}

#[rule]
// convert to shares of 0 assets gives 0 shares.
// status: wip
pub fn convert_to_shares_zero_to_zero(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let assets_are_zero = assets == 0;
    clog!(assets_are_zero);
    let shares_are_zero = shares == 0;
    clog!(shares_are_zero);
    if assets_are_zero {
        cvlr_assert!(shares_are_zero);
    }
}

#[rule]
// convert to assets returns 0 if and only if input is 0
// status: verified
pub fn convert_to_assets_zero_to_zero(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = 0;
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let shares_are_zero = shares == 0;
    clog!(shares_are_zero);
    let assets_are_zero = assets == 0;
    clog!(assets_are_zero);
    cvlr_assert!(shares_are_zero == assets_are_zero);
}

#[rule]
// convert to shares monotonicty
// status: timeout
pub fn convert_to_shares_monotonicity(e: Env) {
    safe_assumptions(&e);
    let assets1: i128 = nondet();
    let assets2: i128 = nondet();
    clog!(assets1);
    clog!(assets2);
    cvlr_assume!(assets1 <= assets2);
    let shares1 = BasicVault::convert_to_shares(&e, assets1);
    let shares2 = BasicVault::convert_to_shares(&e, assets2);
    clog!(shares1);
    clog!(shares2);
    cvlr_assert!(shares1 <= shares2);
}

#[rule]
// convert to assets monotonicity
// status: timeout
pub fn convert_to_assets_monotonicity(e: Env) {
    safe_assumptions(&e);
    let shares1: i128 = nondet();
    let shares2: i128 = nondet();
    clog!(shares1);
    clog!(shares2);
    cvlr_assume!(shares1 <= shares2);
    let assets1 = BasicVault::convert_to_assets(&e, shares1);
    let assets2 = BasicVault::convert_to_assets(&e, shares2);
    clog!(assets1);
    clog!(assets2);
    cvlr_assert!(assets1 <= assets2);
}

#[rule]
// convert to shares weak additivity
// status: timeout
pub fn convert_to_shares_weak_additivity(e: Env) {
    safe_assumptions(&e);
    let assets1: i128 = nondet();
    let assets2: i128 = nondet();
    let assets_sum = assets1 + assets2;
    clog!(assets1);
    clog!(assets2);
    clog!(assets_sum);
    let shares1 = BasicVault::convert_to_shares(&e, assets1);
    let shares2 = BasicVault::convert_to_shares(&e, assets2);
    let shares_sum = BasicVault::convert_to_shares(&e, assets_sum);
    clog!(shares1);
    clog!(shares2);
    clog!(shares_sum);
    cvlr_assert!(shares1 + shares2 <= shares_sum);
}

#[rule]
// convert to assets weak additivity
// status: timeout
pub fn convert_to_assets_weak_additivity(e: Env) {
    safe_assumptions(&e);
    let shares1: i128 = nondet();
    let shares2: i128 = nondet();
    let shares_sum = shares1 + shares2;
    clog!(shares1);
    clog!(shares2);
    clog!(shares_sum);
    let assets1 = BasicVault::convert_to_assets(&e, shares1);
    let assets2 = BasicVault::convert_to_assets(&e, shares2);
    let assets_sum = BasicVault::convert_to_assets(&e, shares_sum);
    clog!(assets1);
    clog!(assets2);
    clog!(assets_sum);
    cvlr_assert!(assets1 + assets2 <= assets_sum);
}

#[rule]
// convert to shares weak inverse
// status: timeout
pub fn convert_to_shares_weak_inverse(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares_from_assets = BasicVault::convert_to_shares(&e, assets);
    clog!(shares_from_assets);
    let assets_from_shares_from_assets = BasicVault::convert_to_assets(&e, shares_from_assets); 
    clog!(assets_from_shares_from_assets);
    cvlr_assert!(assets_from_shares_from_assets <= assets);
}

#[rule]
// convert to assets weak inverse
// status: timeout
pub fn convert_to_assets_weak_inverse(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let assets_from_shares = BasicVault::convert_to_assets(&e, shares);
    clog!(assets_from_shares);
    let shares_from_assets_from_shares = BasicVault::convert_to_shares(&e, assets_from_shares);
    clog!(shares_from_assets_from_shares);
    cvlr_assert!(shares_from_assets_from_shares <= shares);
}
 
#[rule]
// preview_deposit matches convert to shares
// status: timeout
pub fn preview_deposit_matches_convert_to_shares(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let preview_deposit = BasicVault::preview_deposit(&e, assets);
    clog!(preview_deposit);
    cvlr_assert!(preview_deposit == shares);
}

#[rule]
// preview_mint matches convert to assets
// status: timeout
pub fn preview_mint_matches_convert_to_assets(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let preview_mint = BasicVault::preview_mint(&e, shares);
    clog!(preview_mint);
    cvlr_assert!(preview_mint == assets);
}

#[rule]
// preview_withdraw matches convert to shares
// status: timeout
pub fn preview_withdraw_matches_convert_to_shares(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let preview_withdraw = BasicVault::preview_withdraw(&e, assets);
    clog!(preview_withdraw);
    cvlr_assert!(preview_withdraw == shares);
}

#[rule]
// preview_redeem matches convert to assets
// status: timeout
pub fn preview_redeem_matches_convert_to_assets(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let preview_redeem = BasicVault::preview_redeem(&e, shares);
    clog!(preview_redeem);
    cvlr_assert!(preview_redeem == assets);
}

#[rule]
// deposit matches preview_deposit
// status: timeout
pub fn deposit_matches_preview_deposit(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let preview_deposit = BasicVault::preview_deposit(&e, assets);
    clog!(preview_deposit);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let shares = BasicVault::deposit(&e, assets, receiver, from, operator);
    clog!(shares);
    cvlr_assert!(shares == preview_deposit);
}

#[rule]
// withdraw matches preview_withdraw
// status: timeout
pub fn withdraw_matches_preview_withdraw(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let preview_withdraw = BasicVault::preview_withdraw(&e, assets);
    clog!(preview_withdraw);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let shares = BasicVault::withdraw(&e, assets, receiver, owner, operator);
    clog!(shares);
    cvlr_assert!(shares == preview_withdraw);
}

#[rule]
// mint matches preview_mint
// status: timeout
pub fn mint_matches_preview_mint(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let preview_mint = BasicVault::preview_mint(&e, shares);
    clog!(preview_mint);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let assets = BasicVault::mint(&e, shares, receiver, from, operator);
    clog!(assets);
    cvlr_assert!(assets == preview_mint);
}

#[rule]
// redeem matches preview_redeem
// status: timeout
pub fn redeem_matches_preview_redeem(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let preview_redeem = BasicVault::preview_redeem(&e, shares);
    clog!(preview_redeem);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let assets = BasicVault::redeem(&e, shares, receiver, owner, operator);
    clog!(assets);
    cvlr_assert!(assets == preview_redeem);
}