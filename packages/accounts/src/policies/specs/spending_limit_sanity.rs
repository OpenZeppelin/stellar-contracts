use cvlr::{cvlr_assert, nondet::*, cvlr_satisfy};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::{
    policies::spending_limit::{
        can_enforce, enforce, get_spending_limit_data, install, set_spending_limit, uninstall,
        SpendingLimitAccountParams,
    },
    smart_account::ContextRule,
};

#[rule]
pub fn get_spending_limit_sanity(e: Env) {
    let ctx_rule_id: u32 = nondet();
    let account = nondet_address();
    let _ = get_spending_limit_data(&e, ctx_rule_id, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_enforce_spending_limit_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule = ContextRule::nondet();
    let account = nondet_address();
    let _ = can_enforce(&e, &context, &auth_signers, &ctx_rule, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn enforce_spending_limit_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule = ContextRule::nondet();
    let account = nondet_address();
    let _ = enforce(&e, &context, &auth_signers, &ctx_rule, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_spending_limit_sanity(e: Env) {
    let spending_limit_data = nondet();
    let ctx_rule = ContextRule::nondet();
    let account = nondet_address();
    set_spending_limit(&e, spending_limit_data, &ctx_rule, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn install_spending_limit_sanity(e: Env) {
    let params = SpendingLimitAccountParams::nondet();
    let ctx_rule = ContextRule::nondet();
    let account = nondet_address();
    install(&e, &params, &ctx_rule, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn uninstall_spending_limit_sanity(e: Env) {
    let ctx_rule = ContextRule::nondet();
    let account = nondet_address();
    uninstall(&e, &ctx_rule, &account);
    cvlr_satisfy!(true);
}
