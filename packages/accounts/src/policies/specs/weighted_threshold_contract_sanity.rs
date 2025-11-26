use cvlr::{
    cvlr_assert,
    nondet::{self, Nondet},
    cvlr_satisfy,
};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{Policy, weighted_threshold::WeightedThresholdAccountParams, specs::weighted_threshold_contract::WeightedThresholdPolicy},
    smart_account::{ContextRule, Signer},
};

#[rule]
pub fn get_weighted_threshold_sanity(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let _ = WeightedThresholdPolicy::get_threshold(&e, ctx_rule_id, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_signer_weights_weighted_threshold_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let _ = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_enforce_weighted_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = WeightedThresholdPolicy::can_enforce(
        &e,
        context,
        auth_signers,
        ctx_rule,
        account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn enforce_weighted_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = WeightedThresholdPolicy::enforce(
        &e,
        context,
        auth_signers,
        ctx_rule,
        account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_weighted_threshold_sanity(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    WeightedThresholdPolicy::set_threshold(&e, threshold, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_signer_weight_weighted_threshold_sanity(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    WeightedThresholdPolicy::set_signer_weight(&e, signer, weight, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn install_weighted_threshold_sanity(e: Env) {
    let params = WeightedThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    WeightedThresholdPolicy::install(&e, params, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn uninstall_weighted_threshold_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    WeightedThresholdPolicy::uninstall(&e, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

