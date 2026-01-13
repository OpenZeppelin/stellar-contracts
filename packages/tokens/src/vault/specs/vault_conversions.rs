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

pub fn useful_clogs(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    let decimals_offset = Vault::get_decimals_offset(e);
    clog!(decimals_offset);
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
// convert to shares weak inverse
// status: verified
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn convert_to_shares_weak_inverse(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    cvlr_assume!(assets >= i64::MIN as i128 && assets <= i64::MAX as i128);
    clog!(assets);
    let shares_from_assets = BasicVault::convert_to_shares(&e, assets);
    clog!(shares_from_assets);
    let assets_from_shares_from_assets = BasicVault::convert_to_assets(&e, shares_from_assets); 
    clog!(assets_from_shares_from_assets);
    cvlr_assert!(assets_from_shares_from_assets <= assets);
}

#[rule]
// convert to assets weak inverse
// status: verified
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn convert_to_assets_weak_inverse(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    cvlr_assume!(shares >= i64::MIN as i128 && shares <= i64::MAX as i128);
    clog!(shares);
    let assets_from_shares = BasicVault::convert_to_assets(&e, shares);
    clog!(assets_from_shares);
    let shares_from_assets_from_shares = BasicVault::convert_to_shares(&e, assets_from_shares);
    clog!(shares_from_assets_from_shares);
    cvlr_assert!(shares_from_assets_from_shares <= shares);
}
 
#[rule]
// preview_deposit matches convert to shares
// status: verified
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn preview_deposit_matches_convert_to_shares(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    cvlr_assume!(assets >= i64::MIN as i128 && assets <= i64::MAX as i128);
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let preview_deposit = BasicVault::preview_deposit(&e, assets);
    clog!(preview_deposit);
    cvlr_assert!(preview_deposit == shares);
}

#[rule]
// preview_redeem matches convert to assets
// status: verified
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn preview_redeem_matches_convert_to_assets(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    cvlr_assume!(shares >= i64::MIN as i128 && shares <= i64::MAX as i128);
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let preview_redeem = BasicVault::preview_redeem(&e, shares);
    clog!(preview_redeem);
    cvlr_assert!(preview_redeem == assets);
}

#[rule]
// deposit matches preview_deposit
// status: verified
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn deposit_matches_preview_deposit(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    cvlr_assume!(assets >= i64::MIN as i128 && assets <= i64::MAX as i128);
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
