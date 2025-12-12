use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{map, panic_with_error, vec, Env, String, Val, Vec};

use crate::smart_account::{
    authenticate, do_check_auth, get_validated_context,
    specs::{
        nondet::{
            nondet_context, nondet_context_vec, nondet_hash_32, nondet_policy_map,
            nondet_signatures_map, nondet_signers_vec,
        },
        smart_account_contract::SmartAccountContract,
    },
    storage::{self, get_persistent_entry, SmartAccountStorageKey},
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError,
};

#[rule]
pub fn get_context_rule_sanity(e: Env) {
    let id: u32 = nondet();
    let _ = SmartAccountContract::get_context_rule(&e, id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_context_rules_sanity(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let _ = SmartAccountContract::get_context_rules(&e, ctx_typ);
    cvlr_satisfy!(true);
}

#[rule]
pub fn add_context_rule_sanity(e: Env) {
    let ctx_typ = ContextRuleType::nondet();
    let name = nondet_string();
    let valid_until = Option::<u32>::nondet();
    let signers = nondet_signers_vec();
    let policies = nondet_policy_map();
    let _ =
        SmartAccountContract::add_context_rule(&e, ctx_typ, name, valid_until, signers, policies);
    cvlr_satisfy!(true);
}

#[rule]
pub fn update_context_rule_name_sanity(e: Env) {
    let id: u32 = nondet();
    let name = nondet_string();
    let _ = SmartAccountContract::update_context_rule_name(&e, id, name);
    cvlr_satisfy!(true);
}

#[rule]
pub fn update_context_rule_valid_until_sanity(e: Env) {
    let id: u32 = nondet();
    let valid_until = Option::<u32>::nondet();
    let _ = SmartAccountContract::update_context_rule_valid_until(&e, id, valid_until);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_context_rule_sanity(e: Env) {
    let id: u32 = nondet();
    SmartAccountContract::remove_context_rule(&e, id);
    cvlr_satisfy!(true);
}

#[rule]
pub fn add_signer_sanity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    SmartAccountContract::add_signer(&e, id, signer);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_signer_sanity(e: Env) {
    let id: u32 = nondet();
    let signer = Signer::nondet();
    SmartAccountContract::remove_signer(&e, id, signer);
    cvlr_satisfy!(true);
}

#[rule]
pub fn add_policy_sanity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    let install_param = soroban_sdk::Val::from_payload(u64::nondet());
    SmartAccountContract::add_policy(&e, id, policy, install_param);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_policy_sanity(e: Env) {
    let id: u32 = nondet();
    let policy = nondet_address();
    SmartAccountContract::remove_policy(&e, id, policy);
    cvlr_satisfy!(true);
}

#[rule]
pub fn do_check_auth_sanity(e: Env) {
    let signature_payload = nondet_hash_32();
    let signatures = nondet_signatures_map();
    let auth_contexts = nondet_context_vec();
    let _ = do_check_auth(&e, &signature_payload, &signatures, &auth_contexts);
    cvlr_satisfy!(true);
}

#[rule]
pub fn authenticate_sanity(e: Env) {
    let signature_payload = nondet_hash_32();
    let signatures = nondet_signatures_map();
    let _ = authenticate(&e, &signature_payload, &signatures.0);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_validated_context_sanity(e: Env) {
    let context = nondet_context();
    let all_signers = nondet_signers_vec();
    get_validated_context(&e, &context, &all_signers);
    cvlr_satisfy!(true);
}
