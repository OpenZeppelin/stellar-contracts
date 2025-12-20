use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use stellar_contract_utils::math::fixed_point::Rounding;

use crate::{
    fungible::FungibleToken,
    vault::{
        specs::{basic_token::BasicToken, vault::BasicVault},
        FungibleVault, Vault,
    },
};

#[rule]
// deposit changes balances of BasicToken and BasicVault correctly.
// status: timeout
pub fn deposit_integrity(e: Env) {
    let assets: i128 = nondet();
    let receiver: Address = nondet_address();
    let from: Address = nondet_address();
    let operator: Address = nondet_address();
    // let shares_receiver_pre = BasicVault::balance(&e, receiver.clone());
    let assets_from_pre = BasicToken::balance(&e, from.clone());
    let shares = BasicVault::deposit(&e, assets, receiver.clone(), from.clone(), operator.clone());
    // let shares_receiver_post = BasicVault::balance(&e, receiver.clone());
    let assets_from_post = BasicToken::balance(&e, from.clone());
    cvlr_assert!(assets_from_post <= assets_from_pre);
    // cvlr_assert!(shares_receiver_post >= shares_receiver_pre);
    // cvlr_assert!(shares_receiver_post == shares_receiver_pre + shares);
    // cvlr_assert!(assets_from_post == assets_from_pre - assets);
}
