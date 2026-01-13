use cvlr::{clog, cvlr_assert, nondet::*};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::policies::weighted_threshold;
use crate::{
    policies::{
        weighted_threshold::{
            calculate_total_weight, can_enforce, get_signer_weights, get_threshold, install,
            set_signer_weight, set_threshold, uninstall, WeightedThresholdAccountParams,
            WeightedThresholdStorageKey,
        },
    },
    smart_account::{ContextRule, Signer, specs::nondet::{nondet_context, nondet_signers_vec}},
};

// note we verify the rules in this file with:
// "loop_iter": 1 or 2
// "optimistic_loop": true
// meaning we consider only runs where the loops are iterated at most 1/2 times.

#[rule]
// can_enforce returns the expected result: total_weight >= threshold_pre where
// total_weight is the sum of the weights of the authenticated signers
// status: verified
// run: https://prover.certora.com/output/33158/3278037fac7949f481dd6dfa49e52a53
pub fn wt_can_enforce_integrity(e: Env) {
    let context: soroban_sdk::auth::Context = nondet_context();
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id: Address = nondet_address();
    let threshold_pre = get_threshold(&e, ctx_rule.id, &account_id);
    let can_enforce_result = can_enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account_id,
    );
    clog!(threshold_pre);
    clog!(can_enforce_result);
    clog!(ctx_rule.id);
    clog!(cvlr_soroban::Addr(&account_id));
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let total_weight = weighted_threshold::calculate_weight(&e, &auth_signers, &ctx_rule, &account_id);
    clog!(total_weight);
    clog!(threshold_pre);
    let expected_result = total_weight >= threshold_pre;
    clog!(expected_result);
    cvlr_assert!(can_enforce_result == expected_result);
}

// can't write an integrity rule for enforce because it panics if can_enforce
// returns false.

#[rule]
// set_threshold sets the threshold
// status: verified
pub fn wt_set_threshold_integrity(e: Env) {
    let threshold: u32 = u32::nondet();
    clog!(threshold);
    let ctx_rule: ContextRule = ContextRule::nondet();
    clog!(ctx_rule.id);
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    set_threshold(&e, threshold, &ctx_rule, &account_id);
    let threshold_post = get_threshold(&e, ctx_rule.id, &account_id);
    clog!(threshold_post);
    cvlr_assert!(threshold_post == threshold);
}

#[rule]
// set_signer_weight sets the weight for a signer
// status: verified
pub fn wt_set_signer_weight_integrity(e: Env) {
    let signer: Signer = Signer::nondet();
    let weight: u32 = u32::nondet();
    clog!(weight);
    let ctx_rule: ContextRule = ContextRule::nondet();
    clog!(ctx_rule.id);
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    set_signer_weight(&e, &signer, weight, &ctx_rule, &account_id);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    let signer_weight_post = signer_weights.get(signer.clone());
    clog!(signer_weight_post);
    cvlr_assert!(signer_weight_post == Some(weight));
}

#[rule]
// install sets the signer weights and threshold
// status: verified
pub fn wt_install_integrity(e: Env) {
    let params: WeightedThresholdAccountParams = WeightedThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    clog!(ctx_rule.id);
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    install(&e, &params, &ctx_rule, &account_id);
    let threshold_post = get_threshold(&e, ctx_rule.id, &account_id);
    clog!(threshold_post);
    cvlr_assert!(threshold_post == params.threshold);
    let signer_weights = get_signer_weights(&e, &ctx_rule, &account_id);
    cvlr_assert!(signer_weights == params.signer_weights);
}

#[rule]
// after uninstall the account ctx is removed
// status: verified
pub fn wt_uninstall_integrity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    clog!(ctx_rule.id);
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    uninstall(&e, &ctx_rule, &account_id);
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let account_ctx_opt: Option<WeightedThresholdAccountParams> =
        e.storage().persistent().get(&key);
    cvlr_assert!(account_ctx_opt.is_none());
}
