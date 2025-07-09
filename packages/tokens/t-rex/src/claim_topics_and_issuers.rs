use soroban_sdk::{Address, Env, Vec};

use crate::TokenBinder;

pub trait ClaimTopicsAndIssuers: TokenBinder {
    fn linked_tokens(e: &Env) -> Vec<Address>;

    // from `ClaimTopicsRegistry`

    fn add_claim_topic(e: &Env, claim_topic: u32, operator: Address);

    fn remove_claim_topic(e: &Env, claim_topic: u32, operator: Address);

    fn get_claim_topics(e: &Env) -> Vec<u32>;

    // from `TrustedIssuersRegistry`

    fn get_trusted_issuers(e: &Env) -> Vec<Address>;

    fn get_trusted_issuers_for_claim_topic(e: &Env, claim_topic: u32) -> Vec<Address>;

    fn get_trusted_issuer_claim_topics(e: &Env, issuer: Address) -> Vec<u32>;

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool;

    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool;

    fn add_trusted_issuer(e: &Env, issuer: Address, claim_topics: Vec<u32>, operator: Address);

    fn remove_trusted_issuer(e: &Env, issuer: Address, operator: Address);

    fn update_issuer_claim_topics(
        e: &Env,
        issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    );
}
