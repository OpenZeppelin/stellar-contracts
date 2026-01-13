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


// total_assets >= 0
// helpers

pub fn assume_pre_total_assets_geq_zero(e: &Env) {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    cvlr_assume!(total_assets >= 0);
}

