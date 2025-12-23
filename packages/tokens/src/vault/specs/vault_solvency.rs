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
use super::helper::{effective_total_assets, effective_total_supply};

// invariant: effective total assets >= effective total supply

// helpers

pub fn assume_pre_solvency(e: &Env) {
    let total_assets = effective_total_assets(e);
    clog!(total_assets);
    let total_supply = effective_total_supply(e);
    clog!(total_supply);
    cvlr_assume!(total_assets >= total_supply);
}

pub fn assert_post_solvency(e: &Env) {
    let total_assets = effective_total_assets(e);
    clog!(total_assets);
    let total_supply = effective_total_supply(e);
    clog!(total_supply);
    cvlr_assert!(total_assets >= total_supply);
}

#[rule]
// status: violation - spurious - my suspicion is that this is a case where the Vault and Asset have the same address.
// although need to understand how this is modeled in the prover.
pub fn after_transfer_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer(&e, from, to, amount);
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_transfer_from_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer_from(&e, spender, from, to, amount);
    assert_post_solvency(&e);
}

#[rule]
// status: verified
pub fn after_approve_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    BasicVault::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_deposit_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::deposit(&e, assets, receiver, from, operator);
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_mint_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::mint(&e, shares, receiver, from, operator);
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_withdraw_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::withdraw(&e, assets, receiver, owner, operator);
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_redeem_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::redeem(&e, shares, receiver, owner, operator);
    assert_post_solvency(&e);
}

// solvency is obviously not maintained when changing the decimal offset or underlying asset.

// we can check also for the operations on the underlying token, so long as the current contract doesn't send tokens.

#[rule]
// status: violation - underflow - investigate
// but also spurious because shares and assets are the same.
// https://prover.certora.com/output/5771024/27efa3519e8f4a66a957fee0f9ed792d/
pub fn after_token_transfer_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
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
    assert_post_solvency(&e);
}

#[rule]
// status: timeout
pub fn after_token_transfer_from_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
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
    assert_post_solvency(&e);
}

#[rule]
// status: verified
pub fn after_token_approve_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
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
    assert_post_solvency(&e);
}

#[rule]
// status:
pub fn conert_to_shares_and_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let effective_total_assets = effective_total_assets(&e);
    clog!(effective_total_assets);
    let effective_total_supply = effective_total_supply(&e);
    clog!(effective_total_supply);
    let assets: i128 = nondet();
    clog!(assets);
    let shares = BasicVault::convert_to_shares(&e, assets);
    clog!(shares);
    let expected_effective_total_assets = effective_total_assets + assets;
    clog!(expected_effective_total_assets);
    let expected_effective_total_supply = effective_total_supply + shares;
    clog!(expected_effective_total_supply);
    cvlr_assert!(expected_effective_total_assets >= expected_effective_total_supply);
}

#[rule]
// status:
pub fn convert_to_assets_and_solvency(e: Env) {
    safe_assumptions(&e);
    assume_pre_solvency(&e);
    let effective_total_assets = effective_total_assets(&e);
    clog!(effective_total_assets);
    let effective_total_supply = effective_total_supply(&e);
    clog!(effective_total_supply);
    let shares: i128 = nondet();
    clog!(shares);
    let assets = BasicVault::convert_to_assets(&e, shares);
    clog!(assets);
    let expected_effective_total_assets = effective_total_assets + assets;
    clog!(expected_effective_total_assets);
    let expected_effective_total_supply = effective_total_supply + shares;
    clog!(expected_effective_total_supply);
    cvlr_assert!(expected_effective_total_assets >= expected_effective_total_supply);
}