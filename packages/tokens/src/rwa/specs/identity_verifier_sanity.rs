use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::rwa::{
    identity_verifier::{storage as idv_storage, IdentityVerifier},
    specs::identity_verifier::IdentityVerifierContract,
};

#[rule]
pub fn verify_identity_sanity(e: Env) {
    let account = nondet_address();
    IdentityVerifierContract::verify_identity(&e, &account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn recovery_target_sanity(e: Env) {
    let old_account = nondet_address();
    IdentityVerifierContract::recovery_target(&e, &old_account);
    cvlr_satisfy!(true);
}

#[rule]
pub fn set_claim_topics_and_issuers_sanity(e: Env) {
    let claim_topics_and_issuers = nondet_address();
    let operator = nondet_address();
    IdentityVerifierContract::set_claim_topics_and_issuers(&e, claim_topics_and_issuers, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn claim_topics_and_issuers_sanity(e: Env) {
    IdentityVerifierContract::claim_topics_and_issuers(&e);
    cvlr_satisfy!(true);
}

