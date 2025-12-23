use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};

use crate::rwa::identity_claims::{storage as claims_storage, IdentityClaims};

pub struct IdentityClaimsContract;

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
        claims_storage::add_claim(e, topic, scheme, &issuer, &signature, &data, &uri)
    }

    fn get_claim(e: &Env, claim_id: BytesN<32>) -> claims_storage::Claim {
        claims_storage::get_claim(e, &claim_id)
    }

    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>> {
        claims_storage::get_claim_ids_by_topic(e, topic)
    }
}
