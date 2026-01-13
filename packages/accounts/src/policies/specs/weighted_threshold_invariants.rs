use cvlr::{clog, cvlr_assert, cvlr_assume, nondet::*};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Map, Vec};

use crate::policies::weighted_threshold;
use crate::{
    policies::{
        weighted_threshold::{
            calculate_total_weight, can_enforce, enforce, get_signer_weights, get_threshold,
            install, set_signer_weight, set_threshold, uninstall, WeightedThresholdAccountParams,
            WeightedThresholdStorageKey,
        },
    },
    smart_account::{ContextRule, Signer, specs::nondet::{nondet_context, nondet_signers_vec}},
};

// threshold != 0 

// helpers

pub fn assume_pre_threshold_non_zero(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    clog!(threshold);
    cvlr_assume!(threshold != 0);
}

pub fn assert_post_threshold_non_zero(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    clog!(threshold);
    cvlr_assert!(threshold != 0);
}

#[rule]
// status: verified
pub fn wt_after_install_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    install(&e, &WeightedThresholdAccountParams::nondet(), &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
// sanity fails but that is expected
pub fn wt_after_uninstall_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    uninstall(&e, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_set_threshold_threshold_non_zero(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_set_signer_weight_threshold_non_zero(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_can_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    can_enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}


// threshold <= weighted_sum(all) 
// this second invariant is violated for actions of 
// the smart_account but should hold for internal functions of the policy 

// helpers

pub fn assume_pre_threshold_leq_weight_sum(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let weight_sum: u32 = calculate_total_weight(&e, &signer_weights);
    clog!(threshold);
    clog!(weight_sum);
    cvlr_assume!(threshold <= weight_sum);
}

pub fn assert_post_threshold_leq_weight_sum(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let weight_sum: u32 = calculate_total_weight(&e, &signer_weights);
    clog!(threshold);
    clog!(weight_sum);
    cvlr_assert!(threshold <= weight_sum);
}

#[rule]
// status: verified
pub fn wt_after_set_threshold_threshold_leq_weight_sum(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_set_signer_weight_threshold_leq_weight_sum(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_install_threshold_leq_weight_sum(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    install(&e, &WeightedThresholdAccountParams::nondet(), &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
// sanity fails but that is expected
pub fn wt_after_uninstall_threshold_leq_weight_sum(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    uninstall(&e, &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_can_enforce_threshold_leq_weight_sum(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    can_enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_enforce_threshold_leq_weight_sum(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}