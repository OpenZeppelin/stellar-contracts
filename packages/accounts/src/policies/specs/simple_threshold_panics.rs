use core::task::Context;

use cvlr::{
    cvlr_assert, cvlr_assume, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::{is_auth, nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{
        simple_threshold::{
            can_enforce, enforce, get_threshold, install, set_threshold, uninstall,
            SimpleThresholdAccountParams, SimpleThresholdStorageKey,
        },
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
};

#[rule]
// set_threshold_panics if invalid threshold
// status: verified
pub fn set_threshold_panics_if_invalid_threshold(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(threshold == 0 || threshold > ctx_rule.signers.len());
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// set_threshold_panics if unauth
// status: verified
pub fn set_threshold_panics_if_unauth(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// get_threshold_panics if no threshold
// status: verified
pub fn get_threshold_panics_if_no_threshold(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let key = SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule_id);
    let threshold_opt: Option<u32> = e.storage().persistent().get(&key);
    cvlr_assume!(threshold_opt.is_none());
    get_threshold(&e, ctx_rule_id, &account_id);
    cvlr_assert!(false);
}

// can_enforce should never panic

#[rule]
// enforce panics if can_enforce returns false
// status: verified
pub fn enforce_panics_if_can_enforce_returns_false(e: Env, context: soroban_sdk::auth::Context) {
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
// install panics if invalid threshold
// status: verified
pub fn install_panics_if_invalid_threshold(e: Env) {
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let threshold = params.threshold;
    cvlr_assume!(threshold == 0 || threshold > ctx_rule.signers.len());
    install(&e, &params, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// install panics if unauth
// status: verified
pub fn install_panics_if_unauth(e: Env) {
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    install(&e, &params, &ctx_rule, &account_id);
    cvlr_assert!(false);
}

#[rule]
// uninstall panics if unauth
// status: verified
pub fn uninstall_panics_if_unauth(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    uninstall(&e, &ctx_rule, &account_id);
    cvlr_assert!(false);
}
