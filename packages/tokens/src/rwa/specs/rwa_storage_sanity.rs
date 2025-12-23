use cvlr::{cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, String};

use crate::{
    fungible::Base,
    rwa::{RWAStorageKey, RWA},
};

#[rule]
pub fn rwa_version_sanity(e: Env) {
    RWA::version(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_onchain_id_sanity(e: Env) {
    RWA::onchain_id(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_compliance_sanity(e: Env) {
    RWA::compliance(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_identity_verifier_sanity(e: Env) {
    RWA::identity_verifier(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_is_frozen_sanity(e: Env) {
    let user = nondet_address();
    RWA::is_frozen(&e, &user);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_get_frozen_tokens_sanity(e: Env) {
    let user = nondet_address();
    let frozen: i128 = nondet();
    RWA::get_frozen_tokens(&e, &user);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_get_free_tokens_sanity(e: Env) {
    let user = nondet_address();
    RWA::get_free_tokens(&e, &user);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_forced_transfer_sanity(e: Env) {
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    RWA::forced_transfer(&e, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_mint_sanity(e: Env) {
    let to = nondet_address();
    let amount: i128 = nondet();
    RWA::mint(&e, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_burn_sanity(e: Env) {
    let user = nondet_address();
    let amount: i128 = nondet();
    RWA::burn(&e, &user, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_recover_balance_sanity(e: Env) {
    let old_account = nondet_address();
    let new_account = nondet_address();
    RWA::recover_balance(&e, &old_account, &new_account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_set_address_frozen_sanity(e: Env) {
    let user = nondet_address();
    let freeze: bool = nondet();
    RWA::set_address_frozen(&e, &user, freeze);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_freeze_partial_tokens_sanity(e: Env) {
    let user = nondet_address();
    let amount: i128 = nondet();
    RWA::freeze_partial_tokens(&e, &user, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_unfreeze_partial_tokens_sanity(e: Env) {
    let user = nondet_address();
    let amount: i128 = nondet();
    RWA::unfreeze_partial_tokens(&e, &user, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_set_onchain_id_sanity(e: Env) {
    let onchain_id = nondet_address();
    RWA::set_onchain_id(&e, &onchain_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_set_compliance_sanity(e: Env) {
    let compliance = nondet_address();
    RWA::set_compliance(&e, &compliance);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_set_identity_verifier_sanity(e: Env) {
    let identity_verifier = nondet_address();
    RWA::set_identity_verifier(&e, &identity_verifier);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_transfer_sanity(e: Env) {
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    RWA::transfer(&e, &from, &to, amount);
    cvlr_satisfy!(true);
}

#[rule]
pub fn rwa_transfer_from_sanity(e: Env) {
    let spender = nondet_address();
    let from = nondet_address();
    let to = nondet_address();
    let amount: i128 = nondet();
    RWA::transfer_from(&e, &spender, &from, &to, amount);
    cvlr_satisfy!(true);
}
