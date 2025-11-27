use core::task::Context;

use cvlr::{
    cvlr_assert,
    nondet::{self, Nondet},
    cvlr_satisfy,
};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{Policy, simple_threshold::SimpleThresholdAccountParams, specs::simple_threshold_contract::SimpleThresholdPolicy},
    smart_account::{ContextRule, Signer},
};


#[rule]
// after set_threshold the threshold is set to input
// status: verified
pub fn set_threshold_integrity(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SimpleThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
    let threshold_post = SimpleThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id);
    cvlr_assert!(threshold_post == threshold);
}

#[rule]
// can_enforce returns the expected auth_signers.len() >= threshold_pre; 
// not really an intgerity rule because this is a view function
// status: verified
pub fn can_enforce_threshold_integrity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let threshold_pre = SimpleThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    let can_enforce = SimpleThresholdPolicy::can_enforce(&e, context, auth_signers.clone(), ctx_rule.clone(), account_id.clone());
    let expected_result = auth_signers.len() >= threshold_pre;
    cvlr_assert!(can_enforce == expected_result);
}

// can't write an integrity rule for enforce because it panics if can_enforce returns false.

#[rule]
// after install the threshold is set to input
// status: verified
pub fn install_integrity(e: Env) {
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SimpleThresholdPolicy::install(&e, params.clone(), ctx_rule.clone(), account_id.clone());
    let threshold_post = SimpleThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    cvlr_assert!(threshold_post == params.threshold);
}

#[rule]
// after uninstall the threshold is removed
// status: verified
pub fn uninstall_integrity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SimpleThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    let key = crate::policies::simple_threshold::SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let threshold_opt: Option<u32> = e.storage().persistent().get(&key);
    cvlr_assert!(threshold_opt.is_none());
}