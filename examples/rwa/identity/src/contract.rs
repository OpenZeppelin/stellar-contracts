//! Identity Example Contract.
//!
//! Demonstrates composing `IdentityClaims` with `Ownable` access control
//! so that only the identity owner can add or remove claims.

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};
use stellar_access::ownable::{self, Ownable};
use stellar_macros::only_owner;
use stellar_tokens::rwa::identity_claims::{self as claims, Claim, IdentityClaims};

#[contract]
pub struct IdentityContract;

#[contractimpl]
impl IdentityContract {
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    #[only_owner]
    pub fn remove_claim(e: &Env, claim_id: BytesN<32>) {
        claims::remove_claim(e, &claim_id);
    }
}

#[contractimpl]
impl IdentityClaims for IdentityContract {
    #[only_owner]
    fn add_claim(
        e: &Env,
        topic: u32,
        scheme: u32,
        issuer: Address,
        signature: Bytes,
        data: Bytes,
        uri: String,
    ) -> BytesN<32> {
        claims::add_claim(e, topic, scheme, &issuer, &signature, &data, &uri)
    }

    fn get_claim(e: &Env, claim_id: BytesN<32>) -> Claim {
        claims::get_claim(e, &claim_id)
    }

    fn get_claim_ids_by_topic(e: &Env, topic: u32) -> Vec<BytesN<32>> {
        claims::get_claim_ids_by_topic(e, topic)
    }
}

#[contractimpl(contracttrait)]
impl Ownable for IdentityContract {}
