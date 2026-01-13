use core::task::Context;

use cvlr::{clog,
    cvlr_assert, cvlr_assume, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::{is_auth, nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Map, Vec};

use crate::{
    policies::{
        weighted_threshold::{
            calculate_total_weight, can_enforce, enforce, get_signer_weights, get_threshold,
            install, set_signer_weight, set_threshold, uninstall, WeightedThresholdAccountParams,
            WeightedThresholdStorageKey,
        },
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
};
use crate::policies::weighted_threshold;

#[rule]
// set_threshold_panics if invalid threshold (threshold == 0)
// status: verified
pub fn wt_set_threshold_panics_if_threshold_zero(e: Env) {
    let threshold: u32 = 0;
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_threshold_panics if threshold > total_weight
// status: verified
pub fn wt_set_threshold_panics_if_threshold_exceeds_total_weight(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let total_weight = calculate_total_weight(&e, &signer_weights);
    cvlr_assume!(threshold > total_weight);
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_threshold_panics if unauth
// status: verified
pub fn wt_set_threshold_panics_if_unauth(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_threshold_panics if not installed
// status: verified
pub fn wt_set_threshold_panics_if_not_installed(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let params_opt: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);
    cvlr_assume!(params_opt.is_none());
    cvlr_assume!(threshold != 0);
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_signer_weight_panics if threshold > total_weight after update
// status: verified
pub fn wt_set_signer_weight_panics_if_threshold_exceeds_total_weight(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    clog!(weight);
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    clog!(signer_weights.get(signer.clone()));
    let total_weight = calculate_total_weight(&e, &signer_weights);
    clog!(total_weight);
    let threshold = get_threshold(&e, ctx_rule.id, &account_id);
    clog!(threshold);
    cvlr_assume!(threshold > total_weight); // kind of weird where we do the assume after but it makes sense.
    cvlr_assert!(false)
}

#[rule]
// set_signer_weight_panics if unauth
// status: verified
pub fn wt_set_signer_weight_panics_if_unauth(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_signer_weight_panics if not installed
// status: verified
pub fn wt_set_signer_weight_panics_if_not_installed(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let params_opt: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);
    cvlr_assume!(params_opt.is_none());
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// get_threshold_panics if not installed
// status: verified
pub fn wt_get_threshold_panics_if_no_threshold(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule_id);
    let params_opt: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);
    cvlr_assume!(params_opt.is_none());
    get_threshold(&e, ctx_rule_id, &account_id);
    cvlr_assert!(false);
}

#[rule]
// get_signer_weights_panics if not installed
// status: verified
pub fn wt_get_signer_weights_panics_if_not_installed(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let params_opt: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);
    cvlr_assume!(params_opt.is_none());
    get_signer_weights(&e, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

// can_enforce should never panic

#[rule]
// enforce panics if can_enforce returns false
// status: verified
// with loop_iter = 1
// https://prover.certora.com/output/5771024/7fd4fcbf3e4540ebb702bdabeb693a43/?anonymousKey=7da4b4af948e24fac09989e10d0e085bd4dc4f4a
pub fn wt_enforce_panics_if_can_enforce_returns_false(e: Env, context: soroban_sdk::auth::Context) {
    let authenticated_signers: Vec<Signer> = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let can_enforce_result = can_enforce(
        &e,
        &context,
        &authenticated_signers,
        &ctx_rule,
        &account_id,
    );
    cvlr_assume!(!can_enforce_result);
    enforce(&e, &context, &authenticated_signers, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// enforce panics if unauth
// status: verified
pub fn wt_enforce_panics_if_unauth(e: Env, context: soroban_sdk::auth::Context) {
    let authenticated_signers: Vec<Signer> = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    enforce(&e, &context, &authenticated_signers, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// install panics if invalid threshold (threshold == 0)
// status: verified
pub fn wt_install_panics_if_threshold_zero(e: Env) {
    let mut params: WeightedThresholdAccountParams = WeightedThresholdAccountParams::nondet();
    params.threshold = 0;
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    install(&e, &params, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// install panics if threshold > total_weight
// status: verified
pub fn wt_install_panics_if_threshold_exceeds_total_weight(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let params: WeightedThresholdAccountParams = WeightedThresholdAccountParams::nondet();
    install(&e, &params, &ctx_rule, &account_id);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let threshold = get_threshold(&e, ctx_rule.id, &account_id);
    let total_weight = calculate_total_weight(&e, &signer_weights);
    cvlr_assume!(threshold > total_weight);
    cvlr_assert!(false);
}

#[rule]
// install panics if unauth
// status: verified
pub fn wt_install_panics_if_unauth(e: Env) {
    let params: WeightedThresholdAccountParams = WeightedThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    install(&e, &params, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// uninstall panics if unauth
// status: verified
pub fn wt_uninstall_panics_if_unauth(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    uninstall(&e, &ctx_rule, &account_id);
    cvlr_assert!(false);
}
