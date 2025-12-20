use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
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

// integrity rules for all functions of the vault.

// set assets sets the asset adress in storage

// set_decimals_offset sets the decimals offset in storage

#[rule]
// deposit changes decreases the asset balance by assets
// status: timeout
pub fn deposit_integrity_1(e: Env) {
    let assets: i128 = nondet();
    let receiver: Address = nondet_address();
    let from: Address = nondet_address();
    let operator: Address = nondet_address();
    // let shares_receiver_pre = BasicVault::balance(&e, receiver.clone());
    let assets_from_pre = AssetToken::balance(&e, from.clone());
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    // let shares_receiver_post = BasicVault::balance(&e, receiver.clone());
    let assets_from_post = AssetToken::balance(&e, from.clone());
    cvlr_assert!(assets_from_post <= assets_from_pre);
    // cvlr_assert!(shares_receiver_post >= shares_receiver_pre);
    // cvlr_assert!(shares_receiver_post == shares_receiver_pre + shares);
    // cvlr_assert!(assets_from_post == assets_from_pre - assets);
}

// deposit increases the shares balance by shares (output)

// deposit increases total_assets by assets

// deposit increases total_supply by shares

// similar rules for withdraw, mint, redeem

// if these don't work we will write them in terms of the internal functions
// deposit_internal and withdraw_internal