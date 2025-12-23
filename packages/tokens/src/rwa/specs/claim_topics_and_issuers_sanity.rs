use cvlr::{cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::rwa::{
    claim_topics_and_issuers::ClaimTopicsAndIssuers,
    specs::{claim_topics_and_issuers::ClaimTopicsAndIssuersContract, nondet::nondet_vec_u32},
};

#[rule]
pub fn add_claim_topic_sanity(e: Env) {
    let operator = nondet_address();
    let claim_topic: u32 = nondet();
    ClaimTopicsAndIssuersContract::add_claim_topic(&e, claim_topic, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_claim_topic_sanity(e: Env) {
    let operator = nondet_address();
    let claim_topic: u32 = nondet();
    ClaimTopicsAndIssuersContract::remove_claim_topic(&e, claim_topic, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn add_trusted_issuer_sanity(e: Env) {
    let operator = nondet_address();
    let trusted_issuer = nondet_address();
    let topics = nondet_vec_u32();
    ClaimTopicsAndIssuersContract::add_trusted_issuer(&e, trusted_issuer, topics, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn remove_trusted_issuer_sanity(e: Env) {
    let operator = nondet_address();
    let trusted_issuer = nondet_address();
    ClaimTopicsAndIssuersContract::remove_trusted_issuer(&e, trusted_issuer, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn update_issuer_claim_topics_sanity(e: Env) {
    let operator = nondet_address();
    let trusted_issuer = nondet_address();
    let topics = nondet_vec_u32();
    ClaimTopicsAndIssuersContract::update_issuer_claim_topics(&e, trusted_issuer, topics, operator);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_claim_topics_sanity(e: Env) {
    let _ = ClaimTopicsAndIssuersContract::get_claim_topics(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_trusted_issuers_sanity(e: Env) {
    let _ = ClaimTopicsAndIssuersContract::get_trusted_issuers(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_claim_topic_issuers_sanity(e: Env) {
    let claim_topic: u32 = nondet();
    let _ = ClaimTopicsAndIssuersContract::get_claim_topic_issuers(&e, claim_topic);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_claim_topics_and_issuers_sanity(e: Env) {
    let _ = ClaimTopicsAndIssuersContract::get_claim_topics_and_issuers(&e);
    cvlr_satisfy!(true);
}

#[rule]
pub fn get_trusted_issuer_claim_topics_sanity(e: Env) {
    let trusted_issuer = nondet_address();
    let _ = ClaimTopicsAndIssuersContract::get_trusted_issuer_claim_topics(&e, trusted_issuer);
    cvlr_satisfy!(true);
}

#[rule]
pub fn is_trusted_issuer_sanity(e: Env) {
    let issuer = nondet_address();
    let _ = ClaimTopicsAndIssuersContract::is_trusted_issuer(&e, issuer);
    cvlr_satisfy!(true);
}

#[rule]
pub fn has_claim_topic_sanity(e: Env) {
    let issuer = nondet_address();
    let claim_topic: u32 = nondet();
    let _ = ClaimTopicsAndIssuersContract::has_claim_topic(&e, issuer, claim_topic);
    cvlr_satisfy!(true);
}
