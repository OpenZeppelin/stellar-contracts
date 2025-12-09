use cvlr::clog;
use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, String, Val, Vec, map, panic_with_error, vec};

use crate::smart_account::{
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError, specs::{
        nondet::{nondet_policy_map, nondet_signers_vec},
        smart_account_contract::SmartAccountContract,
    }, storage::{self, SmartAccountStorageKey, get_persistent_entry, update_context_rule_valid_until}
};

// functions 
// add_context_rule - hard - failing sanity in 15m

// todo: update_context_rule_name - string equality is hard prob.

#[rule]
// after update_context_rule_valid_until the rule's valid until changes.
// status: verified for loop_iter = 1,2, violated for loop_iter = 3
// what loop??
pub fn update_context_rule_valid_until_integrity(e: Env) {
    let id: u32 = nondet();
    clog!(id);
    let valid_until = Option::<u32>::nondet();
    clog!(valid_until);
    let ctx_rule_pre = SmartAccountContract::get_context_rule(&e, id);
    let valid_until_pre = ctx_rule_pre.valid_until;
    clog!(valid_until_pre);
    update_context_rule_valid_until(&e, id, valid_until);
    let ctx_rule_post = SmartAccountContract::get_context_rule(&e, id);
    let valid_until_post = ctx_rule_post.valid_until;
    clog!(valid_until_post);
    cvlr_assert!(valid_until_post == valid_until);
}

// remove_context_rule
// add_signer
// remove_signer
// add_policy
// remove_policy