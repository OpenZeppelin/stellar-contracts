//! # Identity Claims Contract
//!
//! Manages on-chain identity claims for RWA token compliance.
//! This contract stores and manages claims made by trusted issuers
//! about specific identities.

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};
use stellar_tokens::rwa::identity_claims::{
    add_claim, get_claim, get_claim_ids_by_topic, Claim, IdentityClaims,
};

#[contract]
pub struct IdentityClaimsContract;

#[contractimpl]
impl IdentityClaims for IdentityClaimsContract {
    fn add_claim(
        e: &Env,
        topic: u32,
        scheme: u32,
        issuer: Address,
        signature: Bytes,
        data: Bytes,
        uri: String,
    ) -> BytesN<32> {
        issuer.require_auth();

        add_claim(e, topic, scheme, &issuer, &signature, &data, &uri)
    }

    fn get_claim(e: &Env, claim_id: BytesN<32>) -> Claim {
        get_claim(e, &claim_id)
    }

    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>> {
        get_claim_ids_by_topic(e, topic)
    }
}
