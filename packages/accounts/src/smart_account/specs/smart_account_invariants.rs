use cvlr::{cvlr_assert, cvlr_assume, nondet::*};
use cvlr_soroban_derive::rule;
use cvlr_soroban::{nondet_address, nondet_string};
use soroban_sdk::{Env, String, Val, Vec};

use crate::smart_account::{
    ContextRuleType, MAX_CONTEXT_RULES, Signer, specs::{
        helper::{get_count, get_policies_of_id, get_signers_of_id, validate_signers_and_policies_non_panicking},
        nondet::{nondet_policy_map, nondet_signers_vec},
    }, storage::{
        add_context_rule, add_policy, add_signer, get_context_rule, remove_context_rule, 
        remove_policy, remove_signer, update_context_rule_name, update_context_rule_valid_until,
        ContextRule,
    }
};


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
pub fn after_update_context_rule_name_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let name = nondet_string();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let id_update_context_rule_name = nondet();
    let rule_post = update_context_rule_name(&e, id_update_context_rule_name, &name);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_update_context_rule_valid_until_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let id_update_context_rule_valid_until = nondet();
    let rule_post = update_context_rule_valid_until(&e, id_update_context_rule_valid_until, valid_until);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_remove_context_rule_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let id_remove_context_rule = nondet();
    remove_context_rule(&e, id_remove_context_rule);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

#[rule]
// status: verified
pub fn after_add_signer_valid_signers_and_policies(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    let rule_pre = get_context_rule(&e, id);
    assume_pre_valid_signers_and_policies(e.clone(), rule_pre.clone());
    let id_add_signer = nondet();
    add_signer(&e, id_add_signer, &signer);
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
    let id_remove_signer = nondet();
    remove_signer(&e, id_remove_signer, &signer);
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
    let id_add_policy = nondet();
    add_policy(&e, id_add_policy, &policy, install_param);
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
    let id_remove_policy = nondet();
    remove_policy(&e, id_remove_policy, &policy);
    let rule_post = get_context_rule(&e, id);
    assert_post_valid_signers_and_policies(e.clone(), rule_post);
}

// invariant: number of rules is at most 15

// helpers

pub fn assume_pre_number_of_rules_at_most_max(e: Env) {
    let count = get_count(e);
    let max = MAX_CONTEXT_RULES;
    cvlr_assume!(count <= max);
}

pub fn assert_post_number_of_rules_at_most_max(e: Env) {
    let count = get_count(e);
    let max = MAX_CONTEXT_RULES;
    cvlr_assert!(count <= max);
}

#[rule]
// status: verified
pub fn after_add_context_rule_number_of_rules_at_most_max(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let name = nondet_string();
    let valid_until = Option::<u32>::nondet();
    let signers = nondet_signers_vec();
    let policies = nondet_policy_map();
    assume_pre_number_of_rules_at_most_max(e.clone());
    add_context_rule(&e, &ctx_typ, &name, valid_until, &signers, &policies);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_update_context_rule_name_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let name = nondet_string();
    assume_pre_number_of_rules_at_most_max(e.clone());
    update_context_rule_name(&e, id, &name);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_update_context_rule_valid_until_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    assume_pre_number_of_rules_at_most_max(e.clone());
    update_context_rule_valid_until(&e, id, valid_until);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_remove_context_rule_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    assume_pre_number_of_rules_at_most_max(e.clone());
    remove_context_rule(&e, id);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_add_signer_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    assume_pre_number_of_rules_at_most_max(e.clone());
    add_signer(&e, id, &signer);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_remove_signer_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    assume_pre_number_of_rules_at_most_max(e.clone());
    remove_signer(&e, id, &signer);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_add_policy_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let install_param = Val::from_payload(u64::nondet());
    assume_pre_number_of_rules_at_most_max(e.clone());
    add_policy(&e, id, &policy, install_param);
    assert_post_number_of_rules_at_most_max(e.clone());
}

#[rule]
// status: verified
pub fn after_remove_policy_number_of_rules_at_most_max(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    assume_pre_number_of_rules_at_most_max(e.clone());
    remove_policy(&e, id, &policy);
    assert_post_number_of_rules_at_most_max(e.clone());
}
