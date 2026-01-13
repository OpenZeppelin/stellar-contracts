use core::task::Context;

use cvlr::{
    cvlr_assert, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{
        simple_threshold::{
            can_enforce, get_threshold, install, set_threshold, uninstall,
            SimpleThresholdAccountParams, SimpleThresholdStorageKey,
        },
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
};

#[rule]
// after set_threshold the threshold is set to input
// status: verified
pub fn st_set_threshold_integrity(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    let threshold_post = get_threshold(&e, ctx_rule.id, &account_id);
    cvlr_assert!(threshold_post == threshold);
}

#[rule]
// can_enforce returns the expected auth_signers.len() >= threshold_pre;
// not really an intgerity rule because this is a view function
// status: verified
pub fn st_can_enforce_integrity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let threshold_pre = get_threshold(&e, ctx_rule.id, &account_id);
    let can_enforce_result = can_enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account_id,
    );
    let expected_result = auth_signers.len() >= threshold_pre;
    cvlr_assert!(can_enforce_result == expected_result);
}

// can't write an integrity rule for enforce because it panics if can_enforce
// returns false.

#[rule]
// after install the threshold is set to input
// status: verified
pub fn st_install_integrity(e: Env) {
    let params: SimpleThresholdAccountParams = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    install(&e, &params, &ctx_rule, &account_id);
    let threshold_post = get_threshold(&e, ctx_rule.id, &account_id);
    cvlr_assert!(threshold_post == params.threshold);
}

#[rule]
// after uninstall the account ctx is removed
// status: verified
pub fn st_uninstall_integrity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    uninstall(&e, &ctx_rule, &account_id);
    let key = SimpleThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let account_ctx_opt: Option<u32> = e.storage().persistent().get(&key);
    cvlr_assert!(account_ctx_opt.is_none());
}
