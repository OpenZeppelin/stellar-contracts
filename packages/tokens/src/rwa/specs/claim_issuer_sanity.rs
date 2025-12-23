use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Bytes, Env};

use crate::rwa::{claim_issuer::ClaimIssuer, specs::claim_issuer::ClaimIssuerContract};

#[rule]
pub fn claim_issuer_admin_sanity(e: Env) {
    let identity = nondet_address();
    let scheme: u32 = nondet();
    let claim_topic: u32 = nondet();
    let sig_data = nondet_bytes();
    let claim_data = nondet_bytes();
    // only meaningful once we actually have an implementation of is_claim_valid
    ClaimIssuerContract::is_claim_valid(&e, identity, claim_topic, scheme, sig_data, claim_data);
    cvlr_satisfy!(true);
}
