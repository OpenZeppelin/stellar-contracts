//! # Claim Topics and Issuers Contract
//!
//! Manages claim topics and trusted issuers for RWA token identity verification.
//! This contract defines which claim topics are required and which issuers
//! are trusted to provide claims for those topics.

use soroban_sdk::{contract, contractimpl, contractmeta, symbol_short, Address, Env, Map, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::rwa::claim_topics_and_issuers::{
    emit_claim_topic_added, emit_claim_topic_removed, emit_issuer_topics_updated,
    emit_trusted_issuer_added, emit_trusted_issuer_removed, storage::ClaimTopicsAndIssuersStorage,
    ClaimTopicsAndIssuers,
};

contractmeta!(key = "Description", val = "Claim topics and trusted issuers registry");

/// Role for managing claim topics and issuers
pub const TOPICS_ADMIN_ROLE: soroban_sdk::Symbol = symbol_short!("TOP_ADM");

#[contract]
pub struct ClaimTopicsAndIssuersContract;

#[contractimpl]
impl ClaimTopicsAndIssuers for ClaimTopicsAndIssuersContract {
    #[only_role(operator, "TOP_ADM")]
    fn add_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        ClaimTopicsAndIssuersStorage::add_claim_topic(e, claim_topic);
        emit_claim_topic_added(e, claim_topic);
    }

    #[only_role(operator, "TOP_ADM")]
    fn remove_claim_topic(e: &Env, claim_topic: u32, operator: Address) {
        ClaimTopicsAndIssuersStorage::remove_claim_topic(e, claim_topic);
        emit_claim_topic_removed(e, claim_topic);
    }

    fn get_claim_topics(e: &Env) -> Vec<u32> {
        ClaimTopicsAndIssuersStorage::get_claim_topics(e)
    }

    #[only_role(operator, "TOP_ADM")]
    fn add_trusted_issuer(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    ) {
        ClaimTopicsAndIssuersStorage::add_trusted_issuer(
            e,
            trusted_issuer.clone(),
            claim_topics.clone(),
        );
        emit_trusted_issuer_added(e, &trusted_issuer, claim_topics);
    }

    #[only_role(operator, "TOP_ADM")]
    fn remove_trusted_issuer(e: &Env, trusted_issuer: Address, operator: Address) {
        ClaimTopicsAndIssuersStorage::remove_trusted_issuer(e, trusted_issuer.clone());
        emit_trusted_issuer_removed(e, &trusted_issuer);
    }

    #[only_role(operator, "TOP_ADM")]
    fn update_issuer_claim_topics(
        e: &Env,
        trusted_issuer: Address,
        claim_topics: Vec<u32>,
        operator: Address,
    ) {
        ClaimTopicsAndIssuersStorage::update_issuer_claim_topics(
            e,
            trusted_issuer.clone(),
            claim_topics.clone(),
        );
        emit_issuer_topics_updated(e, &trusted_issuer, claim_topics);
    }

    fn get_trusted_issuers(e: &Env) -> Vec<Address> {
        ClaimTopicsAndIssuersStorage::get_trusted_issuers(e)
    }

    fn get_claim_topic_issuers(e: &Env, claim_topic: u32) -> Vec<Address> {
        ClaimTopicsAndIssuersStorage::get_claim_topic_issuers(e, claim_topic)
    }

    fn get_claim_topics_and_issuers(e: &Env) -> Map<u32, Vec<Address>> {
        ClaimTopicsAndIssuersStorage::get_claim_topics_and_issuers(e)
    }

    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool {
        ClaimTopicsAndIssuersStorage::is_trusted_issuer(e, issuer)
    }

    fn get_trusted_issuer_claim_topics(e: &Env, trusted_issuer: Address) -> Vec<u32> {
        ClaimTopicsAndIssuersStorage::get_trusted_issuer_claim_topics(e, trusted_issuer)
    }

    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool {
        ClaimTopicsAndIssuersStorage::has_claim_topic(e, issuer, claim_topic)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ClaimTopicsAndIssuersContract {}

#[contractimpl]
impl ClaimTopicsAndIssuersContract {
    /// Initializes the contract with an admin
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &admin, &admin, &TOPICS_ADMIN_ROLE);
    }

    /// Sets up common claim topics for KYC/AML compliance
    #[only_role(admin, "TOP_ADM")]
    pub fn setup_default_topics(e: &Env, admin: Address) {
        // Standard claim topics
        // TODO: fix below
        ClaimTopicsAndIssuersStorage::add_claim_topic(e, 1); // KYC
        ClaimTopicsAndIssuersStorage::add_claim_topic(e, 2); // AML
        ClaimTopicsAndIssuersStorage::add_claim_topic(e, 3); // Accredited Investor
        ClaimTopicsAndIssuersStorage::add_claim_topic(e, 4); // Country Verification

        emit_claim_topic_added(e, 1);
        emit_claim_topic_added(e, 2);
        emit_claim_topic_added(e, 3);
        emit_claim_topic_added(e, 4);
    }
}
