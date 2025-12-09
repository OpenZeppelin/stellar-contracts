use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};

use stellar_contract_utils::math::fixed_point::Rounding;

use crate::vault::{FungibleVault, Vault};
use crate::vault::specs::vault::BasicVault;
use crate::vault::specs::basic_token::BasicToken;
use crate::fungible::FungibleToken;

#[rule]
// convert_to_shares(0) returns 0.
// status: verified
pub fn convert_zero_assets(e: Env) {
    let assets: i128 = 0;
    let shares = BasicVault::convert_to_shares(&e, assets);
    cvlr_assert!(shares == 0);
}

#[rule]
// convert_to_assets(0) returns 0. 
// status: verified
pub fn convert_zero_shares(e: Env) {
    let shares: i128 = 0;
    let assets = BasicVault::convert_to_assets(&e, shares);
    cvlr_assert!(assets == 0);
}

// convert zero shares
// do if and only if

// convert to shares monotonicty
// convert to assets monotonicity
// conversion weak additivity
// conversion weak inverse
