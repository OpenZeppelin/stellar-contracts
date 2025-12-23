use soroban_sdk::{contract, contractimpl, Address, Env, Map, Vec};

use crate::rwa::claim_topics_and_issuers::{
    storage, ClaimTopicsAndIssuers, ClaimTopicsAndIssuersError,
};

pub struct ClaimTopicsAndIssuersContract;

impl ClaimTopicsAndIssuers for ClaimTopicsAndIssuersContract {
    fn add_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        operator.require_auth();
        storage::add_claim_topic(e, claim_topic);
    }

    fn remove_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        operator.require_auth();
        storage::remove_claim_topic(e, claim_topic);
    }

    fn get_claim_topics(e: &Env) -> Vec<u32> {
        storage::get_claim_topics(e)
    }

    fn add_trusted_issuer(e: &Env, trusted_issuer: Address, claim_topics: Vec<u32>, operator: Address) {
        operator.require_auth();
        storage::add_trusted_issuer(e, &trusted_issuer, &claim_topics);
    }

    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, operator: Address) {
        operator.require_auth();
        storage::remove_trusted_issuer(e, &trusted_issuer);
    }

    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    ) {
        operator.require_auth();
        storage::update_issuer_claim_topics(e, &trusted_issuer, &claim_topics);
    }

    fn get_trusted_issuers(e: &Env) -> Vec<Address> {
        storage::get_trusted_issuers(e)
    }

    fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address> {
        storage::get_claim_topic_issuers(e, claim_topic)
    }

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool {
        storage::is_trusted_issuer(e, &issuer)
    }

    fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: Address) -> Vec<u32> {
        storage::get_trusted_issuer_claim_topics(e, &trusted_issuer)
    }
    
    fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
       storage::get_claim_topics_and_issuers(e)
    }
    
    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool {
        storage::has_claim_topic(e, &issuer, claim_topic)
    }
}
