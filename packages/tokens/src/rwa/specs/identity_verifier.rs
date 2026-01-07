use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;
use crate::rwa::identity_verifier::storage;
use crate::rwa::identity_verifier::IdentityVerifier;
use crate::rwa::identity_registry_storage;

#[rule]
// after set_claim_topics_and_issuers the claim_topics_and_issuers address is the given input
// status: verified
pub fn set_claim_topics_and_issuers_integrity(e: Env) {
    let claim_topics_and_issuers = nondet_address();
    storage::set_claim_topics_and_issuers(&e, &claim_topics_and_issuers);
    let claim_topics_and_issuers_post = storage::claim_topics_and_issuers(&e);
    cvlr_assert!(claim_topics_and_issuers_post == claim_topics_and_issuers.clone());
}

#[rule]
// after set_identity_registry_storage the identity_registry_storage address is the given input
// status: verified
pub fn set_identity_registry_storage_integrity(e: Env) {
    let identity_registry_storage = nondet_address();
    storage::set_identity_registry_storage(&e, &identity_registry_storage);
    let identity_registry_storage_post = storage::identity_registry_storage(&e);
    cvlr_assert!(identity_registry_storage_post == identity_registry_storage.clone());
}

// the rest of the functions are view functions

#[rule]
// after recovery_target the recovery target is the same as the recovery target in the identity_registry_storage
// status: verified
pub fn recovery_target_matches_identity_registry_storage(e: Env) {
    let old_account = nondet_address();
    let recovery_target = storage::recovery_target(&e, &old_account);
    let recovery_address_from_identity_registry_storage = identity_registry_storage::get_recovered_to(&e, &old_account);
    cvlr_assert!(recovery_target == recovery_address_from_identity_registry_storage);
}

// todo: verify_identity function 
// only panics/non-panics - doesn't return anything.

// todo
// maybe make a non-panicking version of verify_identity

// if there is some invalidity it should panic.

// conjungtive over claims
// disjunctive over issuers
