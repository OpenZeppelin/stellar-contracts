use cvlr::{cvlr_assert, cvlr_assume, nondet::*};
use cvlr_soroban_derive::rule;
use cvlr_soroban::{nondet_address, nondet_string};
use soroban_sdk::{Env, String, Val, Vec};

use crate::smart_account::{
    ContextRuleType, Signer, specs::{
        helper::validate_signers_and_policies_non_panicking,
        nondet::{nondet_policy_map, nondet_signers_vec},
    }, storage::{
        add_context_rule, add_policy, add_signer, get_context_rule, remove_context_rule, 
        remove_policy, remove_signer, update_context_rule_name, update_context_rule_valid_until,
        ContextRule,
    }
};

// todo invariants:
// no duplicate rules
// number of rules is bounded 

// invariant: 
// any ctx rule has at least one signer or a policy <->
// validate_signers_and_policies_non_panicking always returns true

// helpers

pub fn assume_pre_valid_signers_and_policies(e: Env, rule: ContextRule) {
    let signers = rule.signers;
    let policies = rule.policies;
    let valid = validate_signers_and_policies_non_panicking(&e, &signers, &policies);
    cvlr_assume!(valid);
}

pub fn assert_post_valid_signers_and_policies(e: Env, rule: ContextRule) {
    let signers = rule.signers;
    let policies = rule.policies;
    let valid = validate_signers_and_policies_non_panicking(&e, &signers, &policies);
    cvlr_assert!(valid);
}

#[rule]
// status: verified (except for sanity)
pub fn after_add_context_rule_valid_signers_and_policies(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let name = nondet_string();
    let valid_until = Option::<u32>::nondet();
    let signers = nondet_signers_vec();
    let policies = nondet_policy_map();
    let rule = add_context_rule(&e, &ctx_typ, &name, valid_until, &signers, &policies);
    assert_post_valid_signers_and_policies(e.clone(), rule);
}

#[rule]
// status: verified
pub fn after_add_signer_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    add_signer(&e, id, &signer);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified 
pub fn after_remove_signer_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    remove_signer(&e, id, &signer);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified 
pub fn after_add_policy_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let install_param = Val::from_payload(u64::nondet());
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    add_policy(&e, id, &policy, install_param);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_remove_policy_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    remove_policy(&e, id, &policy);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_update_context_rule_name_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let name = nondet_string();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let rule_post = update_context_rule_name(&e, id, &name);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_update_context_rule_valid_until_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let rule_post = update_context_rule_valid_until(&e, id, valid_until);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}