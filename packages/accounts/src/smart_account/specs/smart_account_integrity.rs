use cvlr::clog;
use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address, String, Val, Vec, map, panic_with_error, vec};

use crate::smart_account::{
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError, specs::{
        nondet::{nondet_policy_map, nondet_signers_vec},
        smart_account_contract::SmartAccountContract,
    }, 
};
use crate::smart_account::storage::{
    SmartAccountStorageKey, ContextRule,
    remove_context_rule, get_context_rule, update_context_rule_valid_until, 
    add_signer, remove_signer, add_policy, remove_policy, get_persistent_entry
};

// functions from the trait:

// add_context_rule - hard - failing sanity in 15m

// todo: update_context_rule_name - string equality is hard prob.

#[rule]
// after update_context_rule_valid_until the rule's valid until changes.
// status: verified
// needs loop_iter = 3 for init loop in try_from_val for Meta.
pub fn update_context_rule_valid_until_integrity(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    let ctx_rule_post = update_context_rule_valid_until(&e, id, valid_until);
    let valid_until_post = ctx_rule_post.valid_until;
    cvlr_assert!(valid_until_post == valid_until);
}

#[rule]
// remove context_rule updates the rule count correctly.
// status: verified
// note: 42 minutes
pub fn remove_context_rule_integrity(e: Env) {
    let id: u32 = nondet();
    clog!(id);
    let ctx_rule_pre = get_context_rule(&e, id);
    let rule_count_pre = e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32);
    remove_context_rule(&e, id);
    let rule_count_post = e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32);
    cvlr_assert!(rule_count_post == rule_count_pre - 1);
}

#[rule]
// after add_signer the signer is added.
// status: wip
pub fn add_signer_integrity(e: Env) {
    let id: u32 = nondet();
    clog!(id);
    let signer = Signer::nondet();
    add_signer(&e, id, &signer);
    let ctx_rule_post = get_context_rule(&e, id);
    let signers_post = ctx_rule_post.signers;
    let signers_contains_signer = signers_post.contains(&signer);
    cvlr_assert!(signers_contains_signer);
}

#[rule]
// after remove_signer the signer is removed
// status: wip
pub fn remove_signer_integrity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    remove_signer(&e, id, &signer);
    let ctx_rule_post = get_context_rule(&e, id);
    let signers_post = ctx_rule_post.signers;
    let signers_contains_signer = signers_post.contains(&signer);
    cvlr_assert!(!signers_contains_signer);
}

#[rule]
// after add_policy the policy is added.
// status: wip
pub fn add_policy_integrity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let ctx_rule_pre = get_context_rule(&e, id);
    let policies_pre = ctx_rule_pre.policies;
    let policies_pre_len: u32 = policies_pre.len();
    cvlr_assume!(policies_pre_len == 0);
    let install_param = Val::from_payload(u64::nondet());
    add_policy(&e, id, &policy, install_param);
    let ctx_rule_post = get_context_rule(&e, id);
    let policies_post = ctx_rule_post.policies;
    let policies_post_len: u32 = policies_post.len();
    cvlr_assert!(policies_post_len == 1);
    let last_policy = policies_post.get(policies_post_len - 1).unwrap();
    cvlr_assert!(last_policy == policy);
    // let policies_contains_policy = policies_post.contains(&policy);
    // cvlr_assert!(policies_contains_policy);
}

#[rule]
// after remove_policy the policy is removed
// status: wip
pub fn remove_policy_integrity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    remove_policy(&e, id, &policy);
    let ctx_rule_post = get_context_rule(&e, id);
    let policies_post = ctx_rule_post.policies;
    let policies_contains_policy = policies_post.contains(&policy);
    cvlr_assert!(!policies_contains_policy);
}