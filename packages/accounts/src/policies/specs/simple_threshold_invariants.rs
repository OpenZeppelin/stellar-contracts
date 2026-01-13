use core::task::Context;
use cvlr::clog;
use cvlr::{
    cvlr_assert, cvlr_assume, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{
        simple_threshold::{
            can_enforce, enforce, get_threshold, install, set_threshold, uninstall,
            SimpleThresholdAccountParams,
        },
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
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
// this is the "constructor"
// status: verified
pub fn after_install_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    install(&e, &SimpleThresholdAccountParams::nondet(), &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// this is trivial but it guarantees nothing is left in storage.
// status: verified
// sanity fails but that is expected    
pub fn after_uninstall_threshold_non_zero(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    uninstall(&e, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_set_threshold_threshold_non_zero(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_can_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    can_enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_enforce_threshold_non_zero(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_non_zero(e.clone(), ctx_rule.clone(), account_id.clone());
}

// threshold <= ctx_rule.signers 
// they write that this may be violated
// by adding removing signers by the smart_account

// helpers

pub fn assume_pre_threshold_less_than_signers(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    let signers: Vec<Signer> = ctx_rule.signers;
    let signers_length: u32 = signers.len();
    clog!(threshold);
    clog!(signers_length);
    cvlr_assume!(threshold <= signers_length);
}

pub fn assert_post_threshold_less_than_signers(e: Env, ctx_rule: ContextRule, account_id: Address) {
    let threshold: u32 = get_threshold(&e, ctx_rule.id, &account_id);
    let signers: Vec<Signer> = ctx_rule.signers;
    let signers_length: u32 = signers.len();
    clog!(threshold);
    clog!(signers_length);
    cvlr_assert!(threshold <= signers_length);
}

#[rule]
// status: verified
pub fn after_set_threshold_threshold_leq_signers_length(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    assert_post_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_install_threshold_leq_signers_length(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    let threshold: u32 = u32::nondet();
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams { threshold };
    install(&e, &params, &ctx_rule, &account_id);
    assert_post_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
// sanity fails but that is expected
pub fn after_uninstall_threshold_leq_signers_length(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
    uninstall(&e, &ctx_rule, &account_id);
    assert_post_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_can_enforce_threshold_leq_signers_length(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    can_enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
}

#[rule]
// status: verified
pub fn after_enforce_threshold_leq_signers_length(e: Env, context: soroban_sdk::auth::Context) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    assume_pre_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    enforce(&e, &context, &auth_signers, &ctx_rule, &account_id);
    assert_post_threshold_less_than_signers(e.clone(), ctx_rule.clone(), account_id.clone());
}