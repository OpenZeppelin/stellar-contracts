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
    policies::simple_threshold::SimpleThresholdAccountParams,
    smart_account::{ContextRule, Signer},
};

#[rule]
pub fn get_simple_threshold_sanity(e: Env) {
    let ctx_rule_id: u32 = u32::nondet();
    let account_id = nondet_address();
    let _ = crate::policies::simple_threshold::get_threshold(&e, ctx_rule_id, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn can_enforce_simple_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers: Vec<Signer> = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = crate::policies::simple_threshold::can_enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn enforce_simple_threshold_sanity(e: Env, context: soroban_sdk::auth::Context) {
    let auth_signers = nondet_vec();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account: Address = nondet_address();
    let _ = crate::policies::simple_threshold::enforce(
        &e,
        &context,
        &auth_signers,
        &ctx_rule,
        &account,
    );
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_simple_threshold_sanity(e: Env) {
    let threshold: u32 = u32::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::simple_threshold::set_threshold(&e, threshold, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn install_simple_threshold_sanity(e: Env) {
    let params = SimpleThresholdAccountParams::nondet();
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::simple_threshold::install(&e, &params, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn uninstall_simple_threshold_sanity(e: Env) {
    let ctx_rule: ContextRule = ContextRule::nondet();
    let account_id = nondet_address();
    crate::policies::simple_threshold::uninstall(&e, &ctx_rule, &account_id);
    cvlr_satisfy!(true);
}
