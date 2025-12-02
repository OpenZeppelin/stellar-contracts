use cvlr::{cvlr_assert, clog, nondet::*};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{Policy, specs::weighted_threshold_contract::WeightedThresholdPolicy},
    smart_account::{ContextRule, Signer},
};

use crate::policies::weighted_threshold::WeightedThresholdAccountParams;
use crate::policies::weighted_threshold::WeightedThresholdStorageKey;

// note we verify the rules in this file with:
// "loop_iter": 1 or 2
// "optimistic_loop": true
// meaning we consider only runs where the loops are iterated at most 1/2 times.

#[rule]
// can_enforce returns the expected result: total_weight >= threshold_pre where
// total_weight is the sum of the weights of the authenticated signers
// status: violated - constant propagation issue?
// https://prover.certora.com/output/5771024/b7b57f5270c744cb8a996fddacf1190d?anonymousKey=adbd322ae0dbe71e6f75367c03e67d963d586358&params=%7B%226%22%3A%7B%22index%22%3A0%2C%22ruleCounterExamples%22%3A%5B%7B%22name%22%3A%22rule_output_5.json%22%2C%22selectedRepresentation%22%3A%7B%22label%22%3A%22PRETTY%22%2C%22value%22%3A0%7D%2C%22callResolutionSingleFilter%22%3A%22%22%2C%22variablesFilter%22%3A%22%22%2C%22callTraceFilter%22%3A%22%22%2C%22variablesOpenItems%22%3A%5Btrue%2Ctrue%5D%2C%22callTraceCollapsed%22%3Atrue%2C%22rightSidePanelCollapsed%22%3Afalse%2C%22rightSideTab%22%3A%22%22%2C%22callResolutionSingleCollapsed%22%3Atrue%2C%22viewStorage%22%3Atrue%2C%22variablesExpandedArray%22%3A%22%22%2C%22expandedArray%22%3A%2247-10-12_3-1-1-1-1-1-1-1-1-145_46%22%2C%22orderVars%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22orderParams%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22scrollNode%22%3A%2244%22%2C%22currentPoint%22%3A0%2C%22trackingChildren%22%3A%5B%5D%2C%22trackingParents%22%3A%5B%5D%2C%22trackingOnly%22%3Afalse%2C%22highlightOnly%22%3Afalse%2C%22filterPosition%22%3A0%2C%22singleCallResolutionOpen%22%3A%5B%5D%2C%22snap_drop_1%22%3Anull%2C%22snap_drop_2%22%3Anull%2C%22snap_filter%22%3A%22%22%7D%5D%7D%7D&generalState=%7B%22fileViewOpen%22%3Afalse%2C%22fileViewCollapsed%22%3Atrue%2C%22mainTreeViewCollapsed%22%3Atrue%2C%22callTraceClosed%22%3Afalse%2C%22mainSideNavItem%22%3A%22rules%22%2C%22globalResSelected%22%3Afalse%2C%22isSideBarCollapsed%22%3Afalse%2C%22isRightSideBarCollapsed%22%3Atrue%2C%22selectedFile%22%3A%7B%7D%2C%22fileViewFilter%22%3A%22%22%2C%22mainTreeViewFilter%22%3A%22%22%2C%22contractsFilter%22%3A%22%22%2C%22globalCallResolutionFilter%22%3A%22%22%2C%22currentRuleUiId%22%3A6%2C%22counterExamplePos%22%3A1%2C%22expandedKeysState%22%3A%223-10-1-1-03-1-1-1-1-1-1-1%22%2C%22expandedFilesState%22%3A%5B%5D%2C%22outlinedfilterShared%22%3A%22000000000%22%7D
pub fn wt_can_enforce_integrity(
    e: Env,
    context: soroban_sdk::auth::Context,
    auth_signers: Vec<Signer>,
    ctx_rule: ContextRule,
    account_id: Address,
) {
    let threshold_pre = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    let can_enforce = WeightedThresholdPolicy::can_enforce(&e, context, auth_signers.clone(), ctx_rule.clone(), account_id.clone());
    clog!(threshold_pre);
    clog!(can_enforce);
    clog!(ctx_rule.id);
    clog!(cvlr_soroban::Addr(&account_id));
    let signer_weights = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule.clone(), account_id.clone());
    let mut total_weight: u32 = 0;
    for signer in auth_signers.iter() {
        if let Some(weight) = signer_weights.get(signer.clone()) {
            clog!(weight);
            total_weight = total_weight
                .checked_add(weight)
                .unwrap();
        }
    }
    clog!(total_weight);
    clog!(threshold_pre);
    let expected_result = total_weight >= threshold_pre;
    clog!(expected_result);
    cvlr_assert!(can_enforce == expected_result);
}

// can't write an integrity rule for enforce because it panics if can_enforce returns false.

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
    WeightedThresholdPolicy::set_threshold(&e, threshold, ctx_rule.clone(), account_id.clone());
    let threshold_post = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id);
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
    WeightedThresholdPolicy::set_signer_weight(&e, signer.clone(), weight, ctx_rule.clone(), account_id.clone());
    let signer_weights = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule.clone(), account_id.clone());
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
    WeightedThresholdPolicy::install(&e, params.clone(), ctx_rule.clone(), account_id.clone());
    let threshold_post = WeightedThresholdPolicy::get_threshold(&e, ctx_rule.id, account_id.clone());
    clog!(threshold_post);
    cvlr_assert!(threshold_post == params.threshold);
    let signer_weights = WeightedThresholdPolicy::get_signer_weights(&e, ctx_rule.clone(), account_id.clone());
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
    WeightedThresholdPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    let key = WeightedThresholdStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let account_ctx_opt: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);
    cvlr_assert!(account_ctx_opt.is_none());
}
