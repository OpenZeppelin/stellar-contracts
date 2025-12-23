use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::rwa::identity_claims::IdentityClaims;
use crate::rwa::specs::identity_claims::IdentityClaimsContract;

#[rule]
pub fn identity_claims_add_claim_sanity(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let _ = IdentityClaimsContract::add_claim(&e, topic, scheme, issuer, signature, data, uri);
    cvlr_satisfy!(true);
}

#[rule]
pub fn identity_claims_get_claim_sanity(e: Env) {
    let data = nondet_bytes_n();
    let _ = IdentityClaimsContract::get_claim(&e, data);
    cvlr_satisfy!(true);
}

#[rule]
pub fn identity_claims_get_claim_ids_by_topic_sanity(e: Env) {
    let topic: u32 = nondet();
    let _ = IdentityClaimsContract::get_claim_ids_by_topic(&e, topic);
    cvlr_satisfy!(true);
}
