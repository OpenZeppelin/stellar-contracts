use cvlr::{cvlr_assert, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::smart_account::{
    specs::smart_account_contract::SmartAccountContract, storage, ContextRuleType, Signer,
    SmartAccount,
};

#[rule]
pub fn get_context_rule_sanity(e: Env) {
    let id: u32 = nondet();
    let _ = SmartAccountContract::get_context_rule(&e, id);
    cvlr_assert!(false);
}

#[rule]
pub fn get_context_rules_sanity(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let _ = SmartAccountContract::get_context_rules(&e, ctx_typ);
    cvlr_assert!(false);
}

#[rule]
pub fn add_context_rule_sanity(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let name = nondet_string();
    let valid_until = Option::<u32>::nondet();
    let signers = nondet_vec();
    let policies = nondet_map();
    let _ =
        SmartAccountContract::add_context_rule(&e, ctx_typ, name, valid_until, signers, policies);
    cvlr_assert!(false);
}

#[rule]
pub fn update_context_rule_name_sanity(e: Env) {
    let id: u32 = nondet();
    let name = nondet_string();
    let _ = SmartAccountContract::update_context_rule_name(&e, id, name);
    cvlr_assert!(false);
}

#[rule]
pub fn update_context_rule_valid_until_sanity(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    let _ = SmartAccountContract::update_context_rule_valid_until(&e, id, valid_until);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_context_rule_sanity(e: Env) {
    let id: u32 = nondet();
    SmartAccountContract::remove_context_rule(&e, id);
    cvlr_assert!(false);
}

#[rule]
pub fn add_signer_sanity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    SmartAccountContract::add_signer(&e, id, signer);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_signer_sanity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    SmartAccountContract::remove_signer(&e, id, signer);
    cvlr_assert!(false);
}

#[rule]
pub fn add_policy_sanity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let install_param = soroban_sdk::Val::from_payload(u64::nondet());
    SmartAccountContract::add_policy(&e, id, policy, install_param);
    cvlr_assert!(false);
}

#[rule]
pub fn remove_policy_sanity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    SmartAccountContract::remove_policy(&e, id, policy);
    cvlr_assert!(false);
}
