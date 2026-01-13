use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{map, panic_with_error, vec, Address, Env, String, Val, Vec};

use crate::smart_account::{
    specs::{
        helper::{
            get_count, get_ids_of_rule_type, get_meta_of_id, get_next_id, get_policies_of_id,
            get_signers_of_id,
        },
        nondet::{nondet_policy_map, nondet_signers_vec},
        smart_account_contract::SmartAccountContract,
    },
    storage::{
        add_context_rule, add_policy, add_signer, get_context_rule, get_persistent_entry,
        get_valid_context_rules, remove_context_rule, remove_policy, remove_signer,
        update_context_rule_name, update_context_rule_valid_until, ContextRule,
        SmartAccountStorageKey,
    },
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError,
};

// functions from the trait:

#[rule]
// after add_context_rule the rule_count increases by 1
// status: verified https://prover.certora.com/output/33158/9a7a12d7c8f840768bdcc6ce30a016e2
pub fn add_context_rule_integrity_1(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let name = nondet_string();
    let valid_until = Option::<u32>::nondet();
    let signers = nondet_signers_vec();
    let policies = nondet_policy_map();
    let count_pre = get_count(e.clone());
    add_context_rule(&e, &ctx_typ, &name, valid_until, &signers, &policies);
    let count_post = get_count(e.clone());
    cvlr_assert!(count_post == count_pre + 1);
}

#[rule]
// after update_context_rule_name the name changes
// status: verified
pub fn update_context_rule_name_integrity(e: Env) {
    let id = nondet();
    let name = nondet_string();
    let ctx_rule_pre = get_context_rule(&e, id);
    update_context_rule_name(&e, id, &name);
    let ctx_rule_post = get_context_rule(&e, id);
    let name_post = ctx_rule_post.name;
    cvlr_assert!(name_post == name);
}

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
// note: 80 minutes!
pub fn remove_context_rule_integrity_1(e: Env) {
    let id: u32 = nondet();
    clog!(id);
    let ctx_rule_pre = get_context_rule(&e, id);
    let rule_count_pre = get_count(e.clone());
    remove_context_rule(&e, id);
    let rule_count_post = get_count(e.clone());
    cvlr_assert!(rule_count_post == rule_count_pre - 1);
}

// remove_context_rule should also remove:
// uninstall policies
// clean policies, signers and metadata from storage.
// less important.

#[rule]
// after add_signer the signer is added.
// status: verified
#[rule]
pub fn add_signer_integrity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    let meta = Meta::nondet();

    // with this storage setup the rule verifies
    let signers: Vec<Signer> = Vec::new(&e);
    // perhaps it would also verify if we push one signer e.g
    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);
    
    add_signer(&e, id, &signer);

    let ctx_rule_post = get_context_rule(&e, id);
    cvlr_assert!(ctx_rule_post.signers.contains(&signer));
}

#[rule]
// after remove_signer the signer is removed
// status: verified 
#[rule]
pub fn remove_signer_integrity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();

    let meta = Meta::nondet();
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    // add a single signer because `remove_signer` assumes no duplicates and
    // `add_signer` does not allow duplicates.
    let mut signers = Vec::new(&e);
    signers.push_back(signer.clone());
    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);
    // raz: doesn't make sense to me

    remove_signer(&e, id, &signer);

    let ctx_rule_post = get_context_rule(&e, id);
    cvlr_assert!(!ctx_rule_post.signers.contains(&signer));
}


