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
// status: spurious violation
// check_add that should panic does not panic and instead causes overlfow
// https://prover.certora.com/output/5771024/e691b4e0a96a4f8cbe050d7366b03b8f/?anonymousKey=20b4a38dbe9fbe90f6445ac7aa93f9e4c01524b9&params=%7B%222%22%3A%7B%22index%22%3A0%2C%22ruleCounterExamples%22%3A%5B%7B%22name%22%3A%22rule_output_1.json%22%2C%22selectedRepresentation%22%3A%7B%22label%22%3A%22PRETTY%22%2C%22value%22%3A0%7D%2C%22callResolutionSingleFilter%22%3A%22%22%2C%22variablesFilter%22%3A%22%22%2C%22callTraceFilter%22%3A%22%22%2C%22variablesOpenItems%22%3A%5Btrue%2Ctrue%5D%2C%22callTraceCollapsed%22%3Atrue%2C%22rightSidePanelCollapsed%22%3Afalse%2C%22rightSideTab%22%3A%22%22%2C%22callResolutionSingleCollapsed%22%3Atrue%2C%22viewStorage%22%3Atrue%2C%22variablesExpandedArray%22%3A%22%22%2C%22expandedArray%22%3A%2270-10-12_3-1-119-1-1-1-1-1-158-063_64_70%22%2C%22orderVars%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22orderParams%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22scrollNode%22%3A%2266%22%2C%22currentPoint%22%3A0%2C%22trackingChildren%22%3A%5B%5D%2C%22trackingParents%22%3A%5B%5D%2C%22trackingOnly%22%3Afalse%2C%22highlightOnly%22%3Afalse%2C%22filterPosition%22%3A0%2C%22singleCallResolutionOpen%22%3A%5B%5D%2C%22snap_drop_1%22%3Anull%2C%22snap_drop_2%22%3Anull%2C%22snap_filter%22%3A%22%22%7D%5D%7D%7D&generalState=%7B%22fileViewOpen%22%3Atrue%2C%22fileViewCollapsed%22%3Atrue%2C%22mainTreeViewCollapsed%22%3Atrue%2C%22callTraceClosed%22%3Afalse%2C%22mainSideNavItem%22%3A%22file%22%2C%22globalResSelected%22%3Afalse%2C%22isSideBarCollapsed%22%3Afalse%2C%22isRightSideBarCollapsed%22%3Atrue%2C%22selectedFile%22%3A%7B%22uiID%22%3A%220f1e02%22%2C%22output%22%3A%22.certora_sources%2Ftokens%2Fsrc%2Fvault%2Fstorage.rs%22%2C%22name%22%3A%22storage.rs%22%7D%2C%22fileViewFilter%22%3A%22%22%2C%22mainTreeViewFilter%22%3A%22%22%2C%22contractsFilter%22%3A%22%22%2C%22globalCallResolutionFilter%22%3A%22%22%2C%22currentRuleUiId%22%3A2%2C%22counterExamplePos%22%3A1%2C%22expandedKeysState%22%3A%222-10-1-02-1-1-1-1-1-1-1-1%22%2C%22expandedFilesState%22%3A%5B%22105c9e%22%2C%22d3e16d%22%2C%22043bd9%22%5D%2C%22outlinedfilterShared%22%3A%22000000000%22%7D
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn convert_to_shares_monotonicity(e: Env) {
    safe_assumptions(&e);
    let assets1: i128 = nondet();
    let assets2: i128 = nondet();
    cvlr_assume!(assets1 >= 0);
    cvlr_assume!(assets2 >= 0);
    cvlr_assume!(assets1 >= i64::MIN as i128 && assets1 <= i64::MAX as i128);
    cvlr_assume!(assets2 >= i64::MIN as i128 && assets2 <= i64::MAX as i128);
    clog!(assets1);
    clog!(assets2);
    cvlr_assume!(assets1 <= assets2);
    let shares1 = BasicVault::convert_to_shares(&e, assets1);
    let shares2 = BasicVault::convert_to_shares(&e, assets2);
    clog!(shares1);
    clog!(shares2);
    cvlr_assert!(shares1 <= shares2);
}
// cex: 
// assets1 = 0
// assets2 = 0x7fffffffffffdd0d
// shares1 = 0
// shares2 = -1
// total_supply = 1
// total_assets = 2^127-1

#[rule]
// convert to assets monotonicity
// status: wip
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn convert_to_assets_monotonicity(e: Env) {
    safe_assumptions(&e);
    let shares1: i128 = nondet();
    let shares2: i128 = nondet();
    cvlr_assume!(shares1 >= i64::MIN as i128 && shares1 <= i64::MAX as i128);
    cvlr_assume!(shares2 >= i64::MIN as i128 && shares2 <= i64::MAX as i128);
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
    cvlr_assume!(assets1 >= i64::MIN as i128 && assets1 <= i64::MAX as i128);
    cvlr_assume!(assets2 >= i64::MIN as i128 && assets2 <= i64::MAX as i128);
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
    cvlr_assume!(shares1 >= i64::MIN as i128 && shares1 <= i64::MAX as i128);
    cvlr_assume!(shares2 >= i64::MIN as i128 && shares2 <= i64::MAX as i128);
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
// preview_mint matches convert to assets
// status: violation wip
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn preview_mint_matches_convert_to_assets(e: Env) {
    safe_assumptions(&e);
    let shares: i128 = nondet();
    cvlr_assume!(shares >= i64::MIN as i128 && shares <= i64::MAX as i128);
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let preview_mint = BasicVault::preview_mint(&e, shares);
    clog!(preview_mint);
    cvlr_assert!(preview_mint == assets);
}

#[rule]
// preview_withdraw matches convert to shares
// status: violation wip
// Note the i64 assumption and the virtual offset being set to 0 in `storage.rs` (which is the default value)
pub fn preview_withdraw_matches_convert_to_shares(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    cvlr_assume!(assets >= i64::MIN as i128 && assets <= i64::MAX as i128);
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let preview_withdraw = BasicVault::preview_withdraw(&e, assets);
    clog!(preview_withdraw);
    cvlr_assert!(preview_withdraw == shares);
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

#[rule]
// withdraw matches preview_withdraw
// status: timeout
pub fn withdraw_matches_preview_withdraw(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    cvlr_assume!(assets >= i64::MIN as i128 && assets <= i64::MAX as i128);
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
    cvlr_assume!(shares >= i64::MIN as i128 && shares <= i64::MAX as i128);
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
    cvlr_assume!(shares >= i64::MIN as i128 && shares <= i64::MAX as i128);
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