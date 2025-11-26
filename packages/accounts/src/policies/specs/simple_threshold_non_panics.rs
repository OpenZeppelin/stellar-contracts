use core::task::Context;

use cvlr::{
    cvlr_assert,
    cvlr_assume,
    nondet::{self, Nondet},
    cvlr_satisfy,
};
use cvlr_soroban::{nondet_address, nondet_vec, is_auth};
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{Policy, simple_threshold::SimpleThresholdAccountParams, specs::simple_threshold_contract::SimpleThresholdPolicy},
    smart_account::{ContextRule, Signer},
};

fn storage_setup_threshold(e: Env, ctx_rule_id: u32, account_id: Address) {
    let threshold: u32 = u32::nondet();
    let key = crate::policies::simple_threshold::SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule_id);
    e.storage().persistent().set(&key, &threshold);
    clog!(threshold);
}

// These rules require the prover arg "prover_args": ["-trapAsAssert true"] to consider also panicking paths.

#[rule]
// requires
// valid threshold
// account_id auth
// status: violated - Expected sym to be a valid Val, in vec/vec_len
pub fn set_threshold_non_panic(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    cvlr_assume!(is_auth(account_id.clone()));
    storage_setup_threshold(e.clone(), ctx_rule.id, account_id.clone());
    cvlr_assume!(threshold != 0 && threshold <= ctx_rule.signers.len());
    SimpleThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(true);
}

#[rule]
// requires
// threshold exists 
// status: verified
pub fn get_threshold_non_panic(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    storage_setup_threshold(e.clone(), ctx_rule_id, account_id.clone());
    let key = crate::policies::simple_threshold::SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule_id);
    let threshold_opt: Option<u32> = e.storage().persistent().get(&key);
    cvlr_assume!(threshold_opt.is_some());
    SimpleThresholdPolicy::get_threshold(&e, ctx_rule_id, account_id);
    cvlr_assert!(true);
} 

#[rule]
// requires nothing
// status: violated - Expected sym to be a valid Val, in vec/vec_len
pub fn can_enforce_non_panic(e: Env, context: soroban_sdk::auth::Context) {
    let authenticated_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    storage_setup_threshold(e.clone(), ctx_rule.id, account_id.clone());
    SimpleThresholdPolicy::can_enforce(&e, context, authenticated_signers, ctx_rule, account_id);
    cvlr_assert!(true);
}

#[rule]
// requires
// can_enforce returns true
// status: violated - unreachable - unwrap_failed
pub fn enforce_non_panic(e: Env, context: soroban_sdk::auth::Context) {
    let authenticated_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    storage_setup_threshold(e.clone(), ctx_rule.id, account_id.clone());
    let can_enforce = SimpleThresholdPolicy::can_enforce(&e, context.clone(), authenticated_signers.clone(), ctx_rule.clone(), account_id.clone());
    cvlr_assume!(can_enforce == true);
    SimpleThresholdPolicy::enforce(&e, context, authenticated_signers, ctx_rule, account_id);
    cvlr_assert!(true);
}

#[rule]
// requires
// account_id auth
// valid threshold
// status: violated - Expected sym to be a valid Val, in vec/vec_len
pub fn install_non_panic(e: Env) {
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    storage_setup_threshold(e.clone(), ctx_rule.id, account_id.clone());
    cvlr_assume!(is_auth(account_id.clone()));
    let threshold = params.threshold;
    cvlr_assume!(threshold != 0 && threshold <= ctx_rule.signers.len());
    SimpleThresholdPolicy::install(&e, params, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(true);
}

#[rule]
// requires
// account_id auth
// status: verified
pub fn uninstall_non_panic(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    storage_setup_threshold(e.clone(), ctx_rule.id, account_id.clone());
    cvlr_assume!(is_auth(account_id.clone()));
    SimpleThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    cvlr_assert!(true);
}