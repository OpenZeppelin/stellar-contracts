use cvlr::clog;
use cvlr::{cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_map, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address, String, Val, Vec, map, panic_with_error, vec};
use soroban_sdk::auth::Context;

use crate::smart_account::{
    ContextRuleType, Meta, Signer, SmartAccount, SmartAccountError, specs::{
        nondet::{nondet_policy_map, nondet_signers_vec, nondet_context},
        smart_account_contract::SmartAccountContract,
        policy::{SimpleThresholdPolicyContract},
    }, 
};
use crate::smart_account::storage::{
    SmartAccountStorageKey, ContextRule, can_enforce_all_policies,
};
use crate::policies::Policy;

#[rule]
// can_enforce_all_policies returns the conjunction of can_enforce of each individual policy
// currently the rule is written with the SimpleThresholdPolicyContract
// status: verified
// for 1 policy.
pub fn can_enforce_all_policies_matches_can_enforce(e: Env) {
    let ctx_rule = ContextRule::nondet();
    let context = nondet_context();
    let matched_signers = nondet_signers_vec();
    clog!(matched_signers.len());
    let policies = ctx_rule.policies.clone();
    clog!(policies.len());
    cvlr_assume!(policies.len()==1); // just one policy -- anyway its the threshold contract

    let threshold = SimpleThresholdPolicyContract::get_threshold(&e, ctx_rule.id, e.current_contract_address());
    clog!(threshold);
    // cvlr_assume!(threshold > 0);

    let can_enforce = SimpleThresholdPolicyContract::can_enforce(&e, context.clone(), matched_signers.clone(), ctx_rule.clone(), e.current_contract_address());
    clog!(can_enforce);
    
    let can_enforce_all_policies = can_enforce_all_policies(
        &e, 
        &context, 
        &ctx_rule, 
        &matched_signers);
    clog!(can_enforce_all_policies);
    cvlr_assert!(can_enforce_all_policies == can_enforce);
}

// other functions in storage, that are not exposed by the trait:
// the other entry point is do_check_auth
// do_check_auth -> authenticate
// do_check_auth -> get_validated_context -> can_enforce_all_policies

// get_validated_context
// if there exists a context rule that has can_enforce_all_policies return true we return true
// so context rules are disjunctive over rules and conjunctive over policies
// note as well that if no valid context it panics instead of returning false.

// the way to do it well is have the policies as "nondet" generic policies
// with a can_enforce function that returns values from a fixed (nondet) ghost mapping
// and enforce function based on can_enfroce
// and have two such policies
// then the rule would refer to them. 
// generically you would want say two ctx rules each with 2 policies