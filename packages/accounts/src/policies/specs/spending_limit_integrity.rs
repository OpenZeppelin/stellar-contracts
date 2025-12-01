// use core::task::Context;

use cvlr::{
    cvlr_assert,
    nondet::{self, Nondet},
    cvlr_satisfy,
    cvlr_assume,
};
use cvlr::clog;
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, IntoVal, Vec};
use soroban_sdk::auth::{Context, ContractContext};
use soroban_sdk::symbol_short;
use crate::{
    policies::{Policy, spending_limit::{SpendingLimitAccountParams, SpendingLimitData, SpendingLimitStorageKey}, specs::spending_limit_contract::SpendingLimitPolicy},
    smart_account::{ContextRule, Signer},
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
    SpendingLimitPolicy::set_spending_limit(&e, spending_limit, ctx_rule.clone(), account_id.clone());
    let spending_limit_data_post = SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id);
    let spending_limit_post = spending_limit_data_post.spending_limit;
    cvlr_assert!(spending_limit_post == spending_limit);
}

#[rule]
// status: violated - spurious.
// trying to describe some path where can_enforce returns true.
// should separate these out to a different file 
// and describe all the different possible paths with loop_iter <= 2
// and the trivial paths that return 0.
// possibly we need an invariant that connects the different parameters of the spending limit
pub fn no_previous_transfer_succeeds(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_vec();
    cvlr_assume!(auth_signers.len() > 0);
    let ctx_rule: ContextRule = ContextRule::nondet();
    let from = nondet_address();
    clog!(cvlr_soroban::Addr(&from));
    let to = nondet_address();
    clog!(cvlr_soroban::Addr(&to));
    let amount = i128::nondet();
    clog!(amount);
    let mut args = Vec::new(&e);
    args.push_back(from.into_val(&e));
    args.push_back(to.into_val(&e));
    args.push_back(amount.into_val(&e));
    let contract_context = Context::Contract(ContractContext{
        fn_name: symbol_short!("transfer"),
        args,
        contract: nondet_address(),
    });
    let account_id = nondet_address();
    clog!(cvlr_soroban::Addr(&account_id));
    let spending_limit_data = SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule.id, account_id.clone());
    let spending_limit = spending_limit_data.spending_limit;
    clog!(spending_limit);
    let total_spent = spending_limit_data.cached_total_spent;
    clog!(total_spent);
    cvlr_assume!(total_spent == 0);
    cvlr_assume!(amount <= spending_limit);
    let result = SpendingLimitPolicy::can_enforce(&e, context, auth_signers.clone(), ctx_rule, account_id);
    cvlr_assert!(result == true);
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