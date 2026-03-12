//! RWA Claim Topics and Issuers Example Contract.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Map, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::rwa::claim_topics_and_issuers::{storage as cti, ClaimTopicsAndIssuers};

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct ClaimTopicsAndIssuersContract;

#[contractimpl]
impl ClaimTopicsAndIssuersContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }
}

#[contractimpl(contracttrait)]
impl ClaimTopicsAndIssuers for ClaimTopicsAndIssuersContract {
    #[only_role(operator, "manager")]
    fn add_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        cti::add_claim_topic(e, claim_topic);
    }

    #[only_role(operator, "manager")]
    fn remove_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        cti::remove_claim_topic(e, claim_topic);
    }

    #[only_role(operator, "manager")]
    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    ) {
        cti::add_trusted_issuer(e, &trusted_issuer, &claim_topics);
    }

    #[only_role(operator, "manager")]
    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, operator: Address) {
        cti::remove_trusted_issuer(e, &trusted_issuer);
    }

    #[only_role(operator, "manager")]
    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    ) {
        cti::update_issuer_claim_topics(e, &trusted_issuer, &claim_topics);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ClaimTopicsAndIssuersContract {}
