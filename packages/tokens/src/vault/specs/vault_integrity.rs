use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
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
// integrity rules for all functions of the vault.

#[rule]
// set assets sets the asset adress in storage
// status: verified
pub fn set_asset_integrity(e: Env) {
    let new_asset_address: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&new_asset_address.clone()));
    Vault::set_asset(&e, new_asset_address.clone());
    let asset_address_post = Vault::query_asset(&e);
    clog!(cvlr_soroban::Addr(&asset_address_post));
    cvlr_assert!(asset_address_post == new_asset_address);
}

#[rule]
// set_decimals_offset sets the decimals offset in storage
// status: verified
pub fn set_decimals_offset_integrity(e: Env) {
    let new_decimals_offset: u32 = nondet();
    clog!(new_decimals_offset);
    Vault::set_decimals_offset(&e, new_decimals_offset.clone());
    let decimals_offset_post = Vault::get_decimals_offset(&e);
    clog!(decimals_offset_post);
    cvlr_assert!(decimals_offset_post == new_decimals_offset);
}

#[rule]
// deposit changes decreases the asset balance by assets
// status: timeout
pub fn deposit_integrity_1(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let assets_from_pre = AssetToken::balance(&e, from.clone());
    clog!(assets_from_pre);
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    clog!(shares);
    let assets_from_post = AssetToken::balance(&e, from.clone());
    clog!(assets_from_post);
    cvlr_assert!(assets_from_post <= assets_from_pre);
    // cvlr_assert!(asset_from_post == assets_from_pre - assets);
}

#[rule]
// deposit increases the shares balance by shares (output)
// status:
pub fn deposit_integrity_2(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let shares_receiver_pre = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_pre);
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    clog!(shares);
    let shares_receiver_post = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_post);
    cvlr_assert!(shares_receiver_post >= shares_receiver_pre);
    // cvlr_assert!(shares_receiver_post == shares_receiver_pre + shares);
}

#[rule]
// deposit increases total_assets by assets
// status:
pub fn deposit_integrity_3(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let total_assets_pre = BasicVault::total_assets(&e);
    clog!(total_assets_pre);
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    clog!(shares);
    let total_assets_post = BasicVault::total_assets(&e);
    clog!(total_assets_post);
    cvlr_assert!(total_assets_post >= total_assets_pre);
    // cvlr_assert!(total_assets_post == total_assets_pre + assets);
}

#[rule]
// deposit increases the shares balance by shares (output)
// status:
pub fn deposit_integrity_4(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let shares_receiver_pre = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_pre);
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    clog!(shares);
    let shares_receiver_post = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_post);
    cvlr_assert!(shares_receiver_post >= shares_receiver_pre);
    // cvlr_assert!(shares_receiver_post == shares_receiver_pre + shares);
}

// similar rules for withdraw, mint, redeem

// if these don't work we will write them in terms of the internal functions
// deposit_internal and withdraw_internal

#[rule]
// deposit_internal decreases the asset balance of from by assets
// status: violated - ?
// https://prover.certora.com/output/5771024/e8f44da0afee4942a34883390d752df4/
pub fn deposit_internal_integrity_1(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let assets_from_pre = AssetToken::balance(&e, from.clone());
    clog!(assets_from_pre);
    Vault::deposit_internal(&e, &receiver, assets, shares, &from, &operator);
    let assets_from_post = AssetToken::balance(&e, from.clone());
    clog!(assets_from_post);
    cvlr_assert!(assets_from_post <= assets_from_pre);
    // cvlr_assert!(assets_from_post == assets_from_pre - assets);
}

#[rule]
// deposit_internal increases the shares balance of receiver by shares
// status: bad rule - ignoring self-transfer
pub fn deposit_internal_integrity_2(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let shares_receiver_pre = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_pre);
    Vault::deposit_internal(&e, &receiver, assets, shares, &from, &operator);
    let shares_receiver_post = BasicVault::balance(&e, receiver.clone());
    clog!(shares_receiver_post);
    cvlr_assert!(shares_receiver_post >= shares_receiver_pre);
    // cvlr_assert!(shares_receiver_post == shares_receiver_pre + shares);
}

#[rule]
// deposit_internal increases total_assets by assets
// status:
pub fn deposit_internal_integrity_3(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let total_assets_pre = BasicVault::total_assets(&e);
    clog!(total_assets_pre);
    Vault::deposit_internal(&e, &receiver, assets, shares, &from, &operator);
    let total_assets_post = BasicVault::total_assets(&e);
    clog!(total_assets_post);
    cvlr_assert!(total_assets_post >= total_assets_pre);
    // cvlr_assert!(total_assets_post == total_assets_pre + assets);
}

#[rule]
// deposit_internal increases total_supply by shares
// status: bad rule - ignores self transfer
pub fn deposit_internal_integrity_4(e: Env) {
    safe_assumptions(&e);
    let assets: i128 = nondet();
    clog!(assets);
    let shares: i128 = nondet();
    clog!(shares);
    let receiver: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&receiver));
    let from: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let operator: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&operator));
    let total_supply_pre = BasicVault::total_supply(&e);
    clog!(total_supply_pre);
    Vault::deposit_internal(&e, &receiver, assets, shares, &from, &operator);
    let total_supply_post = BasicVault::total_supply(&e);
    clog!(total_supply_post);
    cvlr_assert!(total_supply_post >= total_supply_pre);
    // cvlr_assert!(total_supply_post == total_supply_pre + shares);
}