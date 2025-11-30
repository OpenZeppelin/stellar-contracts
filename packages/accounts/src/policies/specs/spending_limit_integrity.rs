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
    policies::{Policy, spending_limit::{SpendingLimitAccountParams, SpendingLimitData, SpendingLimitStorageKey}, specs::spending_limit_contract::SpendingLimitPolicy},
    smart_account::{ContextRule, Signer},
};

#[rule]
// after set_spending_limit the spending_limit is set to the input
// status: verified
pub fn sl_set_spending_limit_integrity(e: Env) {
    let spending_limit: i128 = i128::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::set_spending_limit(&e, spending_limit, ctx_rule.clone(), account_id.clone());
    let spending_limit_data_post = SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id);
    let spending_limit_post = spending_limit_data_post.spending_limit;
    cvlr_assert!(spending_limit_post == spending_limit);
}

#[rule]
// TODO
// status: wip 
pub fn sl_can_enforce_integrity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    let result = SpendingLimitPolicy::can_enforce(&e, context, auth_signers, ctx_rule, account_id);
    let expected_result = false;
    cvlr_assert!(result == expected_result);
}

// can't write an integrity rule for enforce because it panics if can_enforce returns false.

#[rule]
// after install the spending_limit_data is set to the input
// status: verified
pub fn sl_install_integrity(e: Env) {
    let params: SpendingLimitAccountParams = SpendingLimitAccountParams::nondet();
    let params_spending_limit = params.spending_limit;
    let params_period_ledgers = params.period_ledgers;
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::install(&e, params.clone(), ctx_rule.clone(), account_id.clone());
    let spending_limit_data_post = SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id);
    let spending_limit_data_post_spending_limit = spending_limit_data_post.spending_limit;
    let spending_limit_data_post_period_ledgers = spending_limit_data_post.period_ledgers;
    cvlr_assert!(spending_limit_data_post_spending_limit == params_spending_limit);
    cvlr_assert!(spending_limit_data_post_period_ledgers == params_period_ledgers);
}

#[rule]
// after uninstall the spending_limit_data is removed
// status: verified
pub fn sl_uninstall_integrity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::uninstall(&e, ctx_rule.clone(), account_id.clone());
    let key: SpendingLimitStorageKey = SpendingLimitStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let account_ctx_opt: Option<SpendingLimitData> = e.storage().persistent().get(&key);
    cvlr_assert!(account_ctx_opt.is_none());
}