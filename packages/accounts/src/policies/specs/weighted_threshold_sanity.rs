use core::task::Context;

use cvlr::{cvlr_assert, nondet::*, cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_map, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{policies::weighted_threshold::*, smart_account::ContextRule};

#[rule]
pub fn get_weighted_threshold_sanity(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let _ = crate::policies::weighted_threshold::get_threshold(&e, ctx_rule_id, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_signer_weights_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let _ = crate::policies::weighted_threshold::get_signer_weights(&e, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn calculate_weight_sanity(e: Env) {
    let signers = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let _ =
        crate::policies::weighted_threshold::calculate_weight(&e, &signers, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_enforce_weighted_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = crate::policies::weighted_threshold::can_enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn enforce_weighted_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = crate::policies::weighted_threshold::enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_weighted_threshold_sanity(e: Env) {
    let threshold: u32 = nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::weighted_threshold::set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_signer_weight_sanity(e: Env) {
    let signer = nondet();
    let weight: u32 = nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::weighted_threshold::set_signer_weight(
        &e,
        &signer,
        weight,
        &ctx_rule,
        &account_id,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn install_weighted_threshold_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let params = WeightedThresholdAccountParams::nondet();
    let account_id = nondet_address();
    crate::policies::weighted_threshold::install(&e, &params, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn uninstall_weighted_threshold_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::weighted_threshold::uninstall(&e, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}
