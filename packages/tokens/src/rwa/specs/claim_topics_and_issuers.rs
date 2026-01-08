use soroban_sdk::{Env, Address};
use cvlr_soroban_derive::rule;  
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use crate::rwa::claim_topics_and_issuers::storage::{
    add_claim_topic, get_claim_topics, remove_claim_topic, 
    get_claim_topic_issuers, get_trusted_issuer_claim_topics, get_trusted_issuers,
    has_claim_topic, is_trusted_issuer, update_issuer_claim_topics,
};
use crate::rwa::claim_topics_and_issuers::ClaimTopicsAndIssuers;
use crate::rwa::specs::helpers::clogs::clog_vec;

// probably need invariants in pre-state

#[rule]
// after add_claim_topic the claim_topic is in claim_topics
// status: verified
pub fn add_claim_topic_integrity_1(e: Env) {
    let claim_topic: u32 = nondet();
    add_claim_topic(&e, claim_topic);
    let claim_topics = get_claim_topics(&e);
    let topics_contains_claim_topic = claim_topics.contains(claim_topic);
    cvlr_assert!(topics_contains_claim_topic);
}

#[rule]
// after add_claim_topic the claim_topic is not in get_trusted_issuer_claim_topics(issuer) for any issuer
// status: verified
pub fn add_claim_topic_integrity_2(e: Env) {
    let claim_topic: u32 = nondet();
    clog!(claim_topic);
    let issuer = nondet_address();
    clog!(cvlr_soroban::Addr(&issuer));
    let issuer_claim_topics_pre = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_claim_topics_pre);
    let claim_topics = get_claim_topics(&e);
    clog_vec(&claim_topics);
    assume_issuer_topics_subset_all_topics(e.clone(), issuer.clone(), claim_topic); // invariant TO BE PROVEN BELOW
    add_claim_topic(&e, claim_topic);
    let issuer_claim_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_claim_topics);
    let topics_contains_claim_topic = issuer_claim_topics.contains(claim_topic);
    clog!(topics_contains_claim_topic);
    cvlr_assert!(!topics_contains_claim_topic);
}

#[rule]
// after add_claim_topic has_claim_topic returns false for any issuer
// status: violation
pub fn add_claim_topic_integrity_3(e: Env) {
    let claim_topic: u32 = nondet();
    let issuer = nondet_address();
    assume_issuer_topics_subset_all_topics(e.clone(), issuer.clone(), claim_topic); // invariant TO BE PROVEN BELOW
    add_claim_topic(&e, claim_topic);
    let has_claim_topic = has_claim_topic(&e, &issuer, claim_topic);
    cvlr_assert!(!has_claim_topic);
}

#[rule]
// after remove_claim_topic the claim_topic is not in claim_topics
// status: violation
pub fn remove_claim_topic_integrity_1(e: Env) {
    let claim_topic: u32 = nondet();
    remove_claim_topic(&e, claim_topic);
    let claim_topics = get_claim_topics(&e);
    let topics_contains_claim_topic = claim_topics.contains(claim_topic);
    cvlr_assert!(!topics_contains_claim_topic);
}

#[rule]
// after remove_claim_topic the claim_topic is not in get_trusted_issuer_claim_topics(issuer) for any issuer
// status: violation
pub fn remove_claim_topic_integrity_2(e: Env) {
    let claim_topic: u32 = nondet();
    let issuer = nondet_address();
    remove_claim_topic(&e, claim_topic);
    let issuer_claim_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    let topics_contains_claim_topic = issuer_claim_topics.contains(claim_topic);
    cvlr_assert!(!topics_contains_claim_topic);
}

#[rule]
// after remove_claim_topic has_claim_topic returns false for any issuer
// status: violation
pub fn remove_claim_topic_integrity_3(e: Env) {
    let claim_topic: u32 = nondet();
    let issuer = nondet_address();
    remove_claim_topic(&e, claim_topic);
    let has_claim_topic = has_claim_topic(&e, &issuer, claim_topic);
    cvlr_assert!(!has_claim_topic);
}

// add_trusted_issuer

// remove_trusted_issuer

// update_issuer_claim_topics

// invariants

// invariant: topic in get_trusted_issuer_claim_topics(issuer) -> topic in get_claim_topics()

pub fn assume_issuer_topics_subset_all_topics(e: Env, issuer: Address, topic: u32) {
    let issuer_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_topics);
    let all_topics = get_claim_topics(&e);
    clog_vec(&all_topics);
    let issuer_topics_contains_topic = issuer_topics.contains(topic);
    let all_topics_contains_topic = all_topics.contains(topic);
    if issuer_topics_contains_topic {
        cvlr_assume!(all_topics_contains_topic);
    }
}

// TODO