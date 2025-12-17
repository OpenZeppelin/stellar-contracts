use cvlr::clog;
use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string, nondet_bytes};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address, String, Val, Vec, map, panic_with_error, vec, Bytes, crypto::Hash};
use soroban_sdk::auth::Context;

use crate::smart_account::{
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError, specs::{
        nondet::{nondet_policy_map, nondet_signers_vec, nondet_context, nondet_hash_32, nondet_signatures_map},
        smart_account_contract::SmartAccountContract,
        policy1::{Policy1},
        policy2::{Policy2},
        dispatcher::can_enforce_dispatch,
        dispatcher::verify_dispatch,
    }, 
};
use crate::smart_account::storage::{
    SmartAccountStorageKey, ContextRule, can_enforce_all_policies, authenticate,
};
use crate::policies::Policy;

// in this file we analyze the do_check_auth function
// which itself is too complex.
// mainly it calls:
// do_check_auth -> authenticate
// do_check_auth -> get_validated_context -> can_enforce_all_policies
// do_check_auth -> enforce

use crate::verifiers::Verifier;

pub fn cast_and_verify(e: &Env, signer: &Signer, signature_payload: &Hash<32>, signature: &Bytes) -> bool {
    if let Signer::External(verifier, key_data) = signer {
        let hash_as_bytes = Bytes::from_array(&e, &signature_payload.to_bytes().to_array());
        let signature_as_bytes_64 = signature.try_into().expect("bytes must have length 64");
        let key_data_as_bytes_32 = key_data.try_into().expect("bytes must have length 32");
        let verify_result = verify_dispatch(&e, hash_as_bytes, key_data_as_bytes_32, signature_as_bytes_64, verifier.clone());
        verify_result
    }
    else {
        // only handling External Signers 
        // todo: handle Delegated signer
        // for that case we need to have an is_auth equivalent
        // for required_auth_for_args
        cvlr_assume!(false);
        false
    }
}

#[rule]
// authenticate panics if some signature is invalid in the underlying verifier.
// status:
pub fn authenticate_panics_if_some_signature_is_invalid(e: Env) {
    let signature_payload = nondet_hash_32();
    let signatures = nondet_signatures_map();
    let signatures_map = signatures.0;
    let signers = signatures_map.keys();
    cvlr_assume!(signers.len() <= 2);
    let first_signer = signers.get(0).unwrap();
    let second_signer = signers.get(1).unwrap();
    let first_signature = signatures_map.get(first_signer.clone()).unwrap();
    let second_signature = signatures_map.get(second_signer.clone()).unwrap();
    let first_signer_verify_result = cast_and_verify(&e, &first_signer, &signature_payload, &first_signature);
    let second_signer_verify_result = cast_and_verify(&e, &second_signer, &signature_payload, &second_signature);
    let both_signers_verify_result = first_signer_verify_result && second_signer_verify_result;
    cvlr_assume!(!both_signers_verify_result);
    authenticate(&e, &signature_payload, &signatures_map);
    cvlr_assert!(false);
}

#[rule]
// can_enforce_all_policies returns the conjunction of can_enforce of each individual policy
// currently the rule is written with the SimpleThresholdPolicyContract
// status: wip
// for 2 policies.
// rule doesn't work currently - need to check the two policies in policies, not Policy1,Policy2.
pub fn can_enforce_all_policies_matches_can_enforce(e: Env) {
    let ctx_rule = ContextRule::nondet();
    let context = nondet_context();
    let matched_signers = nondet_signers_vec();
    clog!(matched_signers.len());
    let policies = ctx_rule.policies.clone();
    clog!(policies.len());
    cvlr_assume!(policies.len()<=2); // at most two policies

    let first_policy = policies.get(0).unwrap();
    let second_policy = policies.get(1).unwrap();
    let can_enforce_first = can_enforce_dispatch(&e, &context, &matched_signers, &ctx_rule, &e.current_contract_address(), first_policy);
    let can_enforce_second = can_enforce_dispatch(&e, &context, &matched_signers, &ctx_rule, &e.current_contract_address(), second_policy);
    let can_enforce_both = can_enforce_first && can_enforce_second;

    let can_enforce_all_policies = can_enforce_all_policies(
        &e, 
        &context, 
        &ctx_rule, 
        &matched_signers);
    clog!(can_enforce_all_policies);
    
    cvlr_assert!(can_enforce_all_policies == can_enforce_both);
}

// todo:
// get_validated_context
// if there exists a context rule that has can_enforce_all_policies return true we return true
// so context rules are disjunctive over rules and conjunctive over policies
// note as well that if no valid context it panics instead of returning false.

// todo: enforce loop?
// todo: do_check_auth?