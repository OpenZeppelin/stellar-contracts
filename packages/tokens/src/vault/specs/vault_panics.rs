use cvlr::{clog, cvlr_assert, cvlr_assume, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::vault::{
    specs::vault::BasicVault,
    storage::VaultStorageKey,
    FungibleVault, Vault, MAX_DECIMALS_OFFSET,
};

#[rule]
// deposit panics if assets < 0 
// status: verified
pub fn deposit_panic_assets_lt_0(e: Env) {
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    cvlr_assume!(assets < 0);
    let _ = BasicVault::deposit(&e, assets, receiver, from, operator);
    cvlr_assert!(false);
}

#[rule]
// deposit panics if assets > max deposit 
// status: verified
pub fn deposit_panic_assets_gt_max_deposit(e: Env) {
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let max_deposit = BasicVault::max_deposit(&e, receiver.clone());
    clog!(max_deposit);
    cvlr_assume!(assets > max_deposit);
    let _ = BasicVault::deposit(&e, assets, receiver, from, operator);
    cvlr_assert!(false);
}

#[rule]
// withdraw panics if assets < 0 
// status: verified
pub fn withdraw_panic_assets_lt_0(e: Env) {
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    cvlr_assume!(assets < 0);
    let _ = BasicVault::withdraw(&e, assets, receiver, owner, operator);
    cvlr_assert!(false);
}

#[rule]
// withdraw panics if assets > max withdraw 
// status: timeout
pub fn withdraw_panic_assets_gt_max_withdraw(e: Env) {
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let max_withdraw = BasicVault::max_withdraw(&e, owner.clone());
    clog!(max_withdraw);
    cvlr_assume!(assets > max_withdraw);
    let _ = BasicVault::withdraw(&e, assets, receiver, owner, operator);
    cvlr_assert!(false);
}

#[rule] 
// mint panics if shares < 0 
// status: verified
pub fn mint_panic_shares_lt_0(e: Env) {
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    cvlr_assume!(shares < 0);
    let _ = BasicVault::mint(&e, shares, receiver, from, operator);
    cvlr_assert!(false);
}

#[rule]
// mint panics if shares > max mint 
// status: verified
pub fn mint_panic_shares_gt_max_mint(e: Env) {
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let max_mint = BasicVault::max_mint(&e, receiver.clone());
    clog!(max_mint);
    cvlr_assume!(shares > max_mint);
    let _ = BasicVault::mint(&e, shares, receiver, from, operator);
    cvlr_assert!(false);
}

#[rule]
// redeem panics if shares < 0 
// status: verified
pub fn redeem_panic_shares_lt_0(e: Env) {
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    cvlr_assume!(shares < 0);
    let _ = BasicVault::redeem(&e, shares, receiver, owner, operator);
    cvlr_assert!(false);
}

#[rule]
// redeem panics if shares > max redeem 
// status: verified
pub fn redeem_panic_shares_gt_max_redeem(e: Env) {
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let owner: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&owner));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let max_redeem = BasicVault::max_redeem(&e, owner.clone());
    clog!(max_redeem);
    cvlr_assume!(shares > max_redeem);
    let _ = BasicVault::redeem(&e, shares, receiver, owner, operator);
    cvlr_assert!(false);
}

#[rule]
// set_asset panics if the asset is already set
// status: verified
pub fn set_asset_panic_asset_already_set(e: Env) {
    let asset: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&asset));
    let storage_has_key = e.storage().instance().has(&VaultStorageKey::AssetAddress);
    clog!(storage_has_key);
    cvlr_assume!(storage_has_key);
    let _ = Vault::set_asset(&e, asset);
    cvlr_assert!(false);
}

#[rule]
// query_asset panics if the asset is not set
// status: 
// status: verified
pub fn query_asset_panic_asset_not_set(e: Env) {
    let storage_has_key = e.storage().instance().has(&VaultStorageKey::AssetAddress);
    clog!(storage_has_key);
    cvlr_assume!(!storage_has_key);
    let _ = Vault::query_asset(&e);
    cvlr_assert!(false);
}

#[rule]
// set_decimals_offset if the offset is already set
// status: verified
pub fn set_decimals_offset_panic_offset_already_set(e: Env) {
    let offset: u32 = nondet();
    clog!(offset);
    let storage_has_key = e.storage().instance().has(&VaultStorageKey::VirtualDecimalsOffset);
    clog!(storage_has_key);
    cvlr_assume!(storage_has_key);
    let _ = Vault::set_decimals_offset(&e, offset);
    cvlr_assert!(false);
}

#[rule]
// set_decimals_offset panics if the offset is greater than MAX_DECIMALS_OFFSET
// status: verified
pub fn set_decimals_offset_panic_offset_gt_max_decimals_offset(e: Env) {
    let offset: u32 = nondet();
    clog!(offset);
    cvlr_assume!(offset > MAX_DECIMALS_OFFSET);
    let _ = Vault::set_decimals_offset(&e, offset);
    cvlr_assert!(false);
}