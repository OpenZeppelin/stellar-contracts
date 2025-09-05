//! # Identity Claims Contract
//!
//! Manages on-chain identity claims for RWA token compliance.
//! This contract stores and manages claims made by trusted issuers
//! about specific identities.

use soroban_sdk::{
    contract, contractimpl, contractmeta, symbol_short, Address, Bytes, BytesN, Env, String, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::rwa::identity_claims::{
    storage::{add_claim, get_claim, get_claim_ids_by_topic, remove_claim, Claim},
    IdentityClaims,
};

contractmeta!(key = "Description", val = "On-chain identity claims storage");

/// Role for managing claims
pub const CLAIMS_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("CLM_ADM");

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
        // Only the identity owner or authorized admin can add claims
        issuer.require_auth();

        add_claim(e, topic, scheme, issuer, signature, data, uri)
    }

    fn get_claim(e: &Env, claim_id: BytesN<32>) -> Claim {
        get_claim(e, &claim_id)
    }

    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>> {
        get_claim_ids_by_topic(e, topic)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for IdentityClaimsContract {}

#[contractimpl]
impl IdentityClaimsContract {
    /// Initializes the identity claims contract
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &admin, &CLAIMS_ADMIN_ROLE);
    }
}
