use cvlr::{cvlr_assert, cvlr_satisfy, cvlr_assume, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use stellar_contract_utils::math::fixed_point::Rounding;
use cvlr::clog;
use crate::{
    fungible::FungibleToken,
    vault::{
        specs::{asset_token::AssetToken, vault::BasicVault},
        FungibleVault, Vault,
    },
};

// decimals <= max_decimals

pub fn safe_assumptions(e: &Env) {
    assume_pre_total_supply_geq_zero(e);
    assume_pre_total_assets_geq_zero(e);
}

// total_supply >= 0
// helpers
pub fn assume_pre_total_supply_geq_zero(e: &Env) {
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    cvlr_assume!(total_supply >= 0);
}

pub fn assert_post_total_supply_geq_zero(e: &Env) {
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    cvlr_assert!(total_supply >= 0);
}

#[rule]
// status: verified
pub fn after_transfer_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer(&e, from, to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_transfer_from_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer_from(&e, spender, from, to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_approve_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    BasicVault::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status:
pub fn after_deposit_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::deposit(&e, assets, receiver, from, operator);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status:
pub fn after_mint_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::mint(&e, shares, receiver, from, operator);
    assert_post_total_supply_geq_zero(&e);
}   

#[rule]
// status:
pub fn after_withdraw_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::withdraw(&e, shares, receiver, owner, operator);
    assert_post_total_supply_geq_zero(&e);
}


#[rule]
// status:
pub fn after_redeem_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::redeem(&e, shares, receiver, owner, operator);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_set_asset_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let asset: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&asset));
    Vault::set_asset(&e, asset);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_set_decimals_offset_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let offset: u32 = nondet();
    clog!(offset);
    Vault::set_decimals_offset(&e, offset);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_transfer_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    AssetToken::transfer(&e, from, to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_transfer_from_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    AssetToken::transfer_from(&e, spender, from, to, amount);
    assert_post_total_supply_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_approve_total_supply_geq_zero(e: Env) {
    assume_pre_total_supply_geq_zero(&e);
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    AssetToken::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_supply_geq_zero(&e);
}

// total_assets >= 0
// helpers

pub fn assume_pre_total_assets_geq_zero(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    cvlr_assume!(total_assets >= 0);
}

pub fn assert_post_total_assets_geq_zero(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    cvlr_assert!(total_assets >= 0);
}

#[rule]
// status: verified
pub fn after_transfer_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer(&e, from, to, amount);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_transfer_from_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    BasicVault::transfer_from(&e, spender, from, to, amount);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_approve_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let owner = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    BasicVault::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status:
pub fn after_deposit_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::deposit(&e, assets, receiver, from, operator);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status:
pub fn after_mint_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::mint(&e, shares, receiver, from, operator);
    assert_post_total_assets_geq_zero(&e);
}   

#[rule]
// status:
pub fn after_withdraw_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::withdraw(&e, shares, receiver, owner, operator);
    assert_post_total_assets_geq_zero(&e);
}


#[rule]
// status:
pub fn after_redeem_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    BasicVault::redeem(&e, shares, receiver, owner, operator);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_set_asset_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let asset: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&asset));
    Vault::set_asset(&e, asset);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_set_decimals_offset_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let offset: u32 = nondet();
    clog!(offset);
    Vault::set_decimals_offset(&e, offset);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_transfer_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    AssetToken::transfer(&e, from, to, amount);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_transfer_from_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount: i128 = nondet();
    clog!(amount);
    AssetToken::transfer_from(&e, spender, from, to, amount);
    assert_post_total_assets_geq_zero(&e);
}

#[rule]
// status: verified
pub fn after_token_approve_total_assets_geq_zero(e: Env) {
    assume_pre_total_assets_geq_zero(&e);
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let spender: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&spender));
    let amount: i128 = nondet();
    clog!(amount);
    let live_until_ledger: u32 = nondet();
    clog!(live_until_ledger);
    AssetToken::approve(&e, owner, spender, amount, live_until_ledger);
    assert_post_total_assets_geq_zero(&e);
}