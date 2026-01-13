// use core::task::Context;

use cvlr::{
    clog, cvlr_assert, cvlr_assume, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{
    auth::{Context, ContractContext},
    symbol_short, Address, Env, IntoVal, Vec,
};

use crate::{
    policies::{
        specs::spending_limit_contract::SpendingLimitPolicy,
        spending_limit::{SpendingLimitAccountParams, SpendingLimitData, SpendingLimitStorageKey},
        Policy,
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
};

// note we verify the rules in this file with:
// "loop_iter": 1 or 2
// "optimistic_loop": true
// meaning we consider only runs where the loops are iterated at most 1/2 times.

#[rule]
// after set_spending_limit the spending_limit is set to the input
// status: verified
pub fn sl_set_spending_limit_integrity(e: Env) {
    let spending_limit: i128 = i128::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::set_spending_limit(
        &e,
        spending_limit,
        ctx_rule.clone(),
        account_id.clone(),
    );
    let spending_limit_data_post =
        SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id);
    let spending_limit_post = spending_limit_data_post.spending_limit;
    cvlr_assert!(spending_limit_post == spending_limit);
}

// can't write an integrity rule for enforce because it panics if can_enforce
// returns false.

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
    let spending_limit_data_post =
        SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id);
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
    let key: SpendingLimitStorageKey =
        SpendingLimitStorageKey::AccountContext(account_id.clone(), ctx_rule.id);
    let account_ctx_opt: Option<SpendingLimitData> = e.storage().persistent().get(&key);
    cvlr_assert!(account_ctx_opt.is_none());
}
