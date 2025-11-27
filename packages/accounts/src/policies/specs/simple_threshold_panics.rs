use core::task::Context;

use cvlr::{
    cvlr_assert,
    cvlr_assume,
    nondet::{self, Nondet},
    cvlr_satisfy,
};
use cvlr_soroban::{nondet_address, nondet_vec, is_auth};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{Policy, simple_threshold::SimpleThresholdAccountParams, specs::simple_threshold_contract::SimpleThresholdPolicy},
    smart_account::{ContextRule, Signer},
};

#[rule]
// set_threshold_panics if invalid threshold
// status: verified 
pub fn set_threshold_panics_if_invalid_threshold(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(threshold == 0 || threshold > ctx_rule.signers.len());
    SimpleThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
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
    SimpleThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(false);
}

#[rule]
// get_threshold_panics if no threshold
// status: verified 
pub fn get_threshold_panics_if_no_threshold(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let key = crate::policies::simple_threshold::SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule_id);
    let threshold_opt: Option<u32> = e.storage().persistent().get(&key);
    cvlr_assume!(threshold_opt.is_none());
    SimpleThresholdPolicy::get_threshold(&e, ctx_rule_id, account_id);
    cvlr_assert!(false);
} 

// can_enforce should never panic

#[rule]
// enforce panics if can_enforce returns false
// status: verified 
pub fn enforce_panics_if_can_enforce_returns_false(e: Env, context: soroban_sdk::auth::Context) {
    let authenticated_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let can_enforce = SimpleThresholdPolicy::can_enforce(&e, context.clone(), authenticated_signers.clone(), ctx_rule.clone(), account_id.clone());
    cvlr_assume!(can_enforce == false);
    SimpleThresholdPolicy::enforce(&e, context, authenticated_signers, ctx_rule, account_id);
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
    SimpleThresholdPolicy::install(&e, params, ctx_rule.clone(), account_id.clone());
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
    SimpleThresholdPolicy::install(&e, params, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(false);
}


#[rule]
// uninstall panics if unauth
// status: verified 
pub fn uninstall_panics_if_unauth(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(!is_auth(account_id.clone()));
    SimpleThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(false);
}