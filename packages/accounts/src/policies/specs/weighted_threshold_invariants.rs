use cvlr::{clog, cvlr_assert, cvlr_assume, nondet::*};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Map, Vec};

use crate::policies::weighted_threshold;
use crate::{
    policies::{
        Policy, specs::weighted_threshold_contract::WeightedThresholdPolicy, weighted_threshold::{WeightedThresholdAccountParams, WeightedThresholdStorageKey}
    },
    smart_account::{ContextRule, Signer, specs::nondet::{nondet_context, nondet_signers_vec}},
};

// threshold != 0 

// helpers

pub fn assume_pre_threshold_non_zero(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    clog!(threshold);
    cvlr_assume!(threshold != 0);
}

pub fn assert_post_threshold_non_zero(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    clog!(threshold);
    cvlr_assert!(threshold != 0);
}

#[rule]
// status: verified
pub fn wt_after_install_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    WeightedThresholdPolicy::install(&e, WeightedThresholdAccountParams::nondet(), ctx_rule.clone(), account_id.clone());
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
// sanity fails but that is expected
pub fn wt_after_uninstall_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    WeightedThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_set_threshold_threshold_non_zero(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    WeightedThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
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
    WeightedThresholdPolicy::set_signer_weight(&e, signer, weight, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_can_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    let can_enforce = WeightedThresholdPolicy::can_enforce(&e, context, auth_signers, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    WeightedThresholdPolicy::enforce(&e, context, auth_signers, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}


// threshold <= weighted_sum(all) 
// this second invariant is violated for actions of 
// the smart_account but should hold for internal functions of the policy 

// helpers

pub fn assume_pre_threshold_leq_weight_sum(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    let signer_weights = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule.clone(), account_id.clone());
    let weight_sum: u32 = weighted_threshold::calculate_total_weight(&e, &signer_weights);
    clog!(threshold);
    clog!(weight_sum);
    cvlr_assume!(threshold <= weight_sum);
}

pub fn assert_post_threshold_leq_weight_sum(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    let signer_weights = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule.clone(), account_id.clone());
    let weight_sum: u32 = weighted_threshold::calculate_total_weight(&e, &signer_weights);
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
    WeightedThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
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
    WeightedThresholdPolicy::set_signer_weight(&e, signer, weight, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_install_threshold_leq_weight_sum(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    WeightedThresholdPolicy::install(&e, WeightedThresholdAccountParams::nondet(), ctx_rule.clone(), account_id.clone());
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
// sanity fails but that is expected
pub fn wt_after_uninstall_threshold_leq_weight_sum(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    WeightedThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_after_can_enforce_threshold_leq_weight_sum(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    let can_enforce = WeightedThresholdPolicy::can_enforce(&e, context, auth_signers, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn wt_enforce_threshold_leq_weight_sum(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    WeightedThresholdPolicy::enforce(&e, context, auth_signers, ctx_rule.clone(), account_id.clone());
    assert_post_threshold_leq_weight_sum(e.clone(), ctx_rule.clone(), account_id.clone());
}