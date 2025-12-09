use cvlr::{
    cvlr_assert, cvlr_satisfy,
    nondet::{self, Nondet},
};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    policies::{
        specs::spending_limit_contract::SpendingLimitPolicy,
        spending_limit::SpendingLimitAccountParams, Policy,
    },
    smart_account::{specs::nondet::nondet_signers_vec, ContextRule, Signer},
};

#[rule]
pub fn get_spending_limit_data_sanity(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let _ = SpendingLimitPolicy::get_spending_limit_data(&e, ctx_rule_id, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_enforce_spending_limit_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = SpendingLimitPolicy::can_enforce(&e, context, auth_signers, ctx_rule, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enforce_spending_limit_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_signers_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    SpendingLimitPolicy::enforce(&e, context, auth_signers, ctx_rule, account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_spending_limit_sanity(e: Env) {
    let spending_limit: i128 = i128::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::set_spending_limit(&e, spending_limit, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn install_spending_limit_sanity(e: Env) {
    let params = SpendingLimitAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::install(&e, params, ctx_rule, account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn uninstall_spending_limit_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    SpendingLimitPolicy::uninstall(&e, ctx_rule, account_id);
    cvlr_satisfy!(true);
}
