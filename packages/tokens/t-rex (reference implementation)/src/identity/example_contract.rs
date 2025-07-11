use crate::identity::registry::IdentityRegistry;
use crate::identity::verifier::IdentityVerifier;
use soroban_sdk::{Address, Env, Vec};

pub struct ExampleContract;

impl IdentityVerifier for ExampleContract {
    fn is_verified(e: &Env, account: Address) -> bool {
        match self::IdentityRegistry::get_identity_onchain_id(e, account) {
            Some(contract) => {
                let claim_topics = self::IdentityRegistry::get_claim_topics(e);

                // cross contract call for `get_claims` from `contract`
                let claims = get_claims(e);

                for claim_topic in claim_topics {
                    // if claim_topic does not exist in claims, return false
                    if !claims.contains_key(claim_topic) {
                        return false;
                    }
                }
            }
            None => false,
        }
    }
}

impl IdentityRegistry for ExampleContract {
    fn add_identity(
        e: &Env,
        operator: Address,
        user_address: Address,
        identity_contract: Address,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_identity_onchain_id(e: &Env, user_address: Address) -> Option<Address> {
        unimplemented!()
    }

    fn remove_identity(e: &Env, operator: Address, user_address: Address) -> Result<(), Error> {
        unimplemented!()
    }

    fn modify_identity(
        e: &Env,
        operator: Address,
        user_address: Address,
        new_identity_contract: Address,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    fn add_claim_topic(
        e: &Env,
        operator: Address,
        claim_topic: u32,
    ) -> Result<(), super::registry::Error> {
        unimplemented!()
    }

    fn remove_claim_topic(
        e: &Env,
        operator: Address,
        claim_topic: u32,
    ) -> Result<(), super::registry::Error> {
        unimplemented!()
    }

    fn get_claim_topics(e: &Env) -> Vec<u32> {
        unimplemented!()
    }

    fn add_trusted_issuer(
        e: &Env,
        operator: Address,
        issuer: Address,
        claim_topics: Vec<u32>,
    ) -> Result<(), super::registry::Error> {
        unimplemented!()
    }

    fn remove_trusted_issuer(
        e: &Env,
        operator: Address,
        issuer: Address,
    ) -> Result<(), super::registry::Error> {
        unimplemented!()
    }

    fn update_issuer_claim_topics(
        e: &Env,
        operator: Address,
        issuer: Address,
        claim_topics: Vec<u32>,
    ) -> Result<(), super::registry::Error> {
        unimplemented!()
    }

    fn get_trusted_issuers(e: &Env) -> Vec<Address> {
        unimplemented!()
    }

    fn get_trusted_issuers_for_claim_topic(e: &Env, claim_topic: u32) -> Vec<Address> {
        unimplemented!()
    }

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool {
        unimplemented!()
    }

    fn get_trusted_issuer_claim_topics(
        e: &Env,
        issuer: Address,
    ) -> Result<Vec<u32>, super::registry::Error> {
        unimplemented!()
    }

    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool {
        unimplemented!()
    }
}
