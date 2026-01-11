use soroban_sdk::{Env, Address, Vec};
use cvlr_soroban_derive::rule;  
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use crate::rwa::claim_topics_and_issuers::storage::{
    add_claim_topic, get_claim_topics, remove_claim_topic, 
    get_claim_topic_issuers, get_trusted_issuer_claim_topics, get_trusted_issuers,
    has_claim_topic, is_trusted_issuer, update_issuer_claim_topics, add_trusted_issuer,
    remove_trusted_issuer
};
use crate::rwa::claim_topics_and_issuers::ClaimTopicsAndIssuers;
use crate::rwa::specs::helpers::nondet::nondet_vec_u32;
use crate::rwa::specs::helpers::clogs::{clog_vec, clog_vec_addresses};

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
    assume_pre_issuer_topics_subset_all_topics(e.clone(), issuer.clone(), claim_topic); // invariant TO BE PROVEN BELOW
    add_claim_topic(&e, claim_topic);
    let issuer_claim_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_claim_topics);
    let topics_contains_claim_topic = issuer_claim_topics.contains(claim_topic);
    clog!(topics_contains_claim_topic);
    cvlr_assert!(!topics_contains_claim_topic);
}

#[rule]
// after add_claim_topic has_claim_topic returns false for any issuer
// status: verified
pub fn add_claim_topic_integrity_3(e: Env) {
    let claim_topic: u32 = nondet();
    clog!(claim_topic);
    let issuer = nondet_address();
    clog!(cvlr_soroban::Addr(&issuer));
    assume_pre_issuer_topics_subset_all_topics(e.clone(), issuer.clone(), claim_topic); // invariant TO BE PROVEN BELOW
    add_claim_topic(&e, claim_topic);
    let has_claim_topic = has_claim_topic(&e, &issuer, claim_topic);
    clog!(has_claim_topic);
    cvlr_assert!(!has_claim_topic);
}

#[rule]
// after remove_claim_topic the claim_topic is not in claim_topics
// status: verified
pub fn remove_claim_topic_integrity_1(e: Env) {
    let claim_topic: u32 = nondet();
    clog!(claim_topic);
    clog_vec(&get_claim_topics(&e));
    assume_pre_no_duplicate_topics(e.clone()); // invariant TO BE PROVEN BELOW
    remove_claim_topic(&e, claim_topic);
    let claim_topics = get_claim_topics(&e);
    clog_vec(&claim_topics);
    let topics_contains_claim_topic = claim_topics.contains(claim_topic);
    clog!(topics_contains_claim_topic);
    cvlr_assert!(!topics_contains_claim_topic);
}

#[rule]
// after remove_claim_topic the claim_topic is not in get_trusted_issuer_claim_topics(issuer) for any issuer
// status: spurious violation
// https://prover.certora.com/output/5771024/d0891843db7745738eb679099935ade6/?&params=%7B%221%22%3A%7B%22index%22%3A0%2C%22ruleCounterExamples%22%3A%5B%7B%22name%22%3A%22rule_output_1.json%22%2C%22selectedRepresentation%22%3A%7B%22label%22%3A%22PRETTY%22%2C%22value%22%3A0%7D%2C%22callResolutionSingleFilter%22%3A%22%22%2C%22variablesFilter%22%3A%22%22%2C%22callTraceFilter%22%3A%22%22%2C%22variablesOpenItems%22%3A%5Btrue%2Ctrue%5D%2C%22callTraceCollapsed%22%3Atrue%2C%22rightSidePanelCollapsed%22%3Afalse%2C%22rightSideTab%22%3A%22%22%2C%22callResolutionSingleCollapsed%22%3Atrue%2C%22viewStorage%22%3Atrue%2C%22variablesExpandedArray%22%3A%22%22%2C%22expandedArray%22%3A%22501-10-138_43_44-152_91_96_97-1105-1-1205-1288_293_294-1-1-1-1486_491_492_500%22%2C%22orderVars%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22orderParams%22%3A%5B%22%22%2C%22%22%2C0%5D%2C%22scrollNode%22%3A%2255%22%2C%22currentPoint%22%3A0%2C%22trackingChildren%22%3A%5B%5D%2C%22trackingParents%22%3A%5B%5D%2C%22trackingOnly%22%3Afalse%2C%22highlightOnly%22%3Afalse%2C%22filterPosition%22%3A0%2C%22singleCallResolutionOpen%22%3A%5B%5D%2C%22snap_drop_1%22%3Anull%2C%22snap_drop_2%22%3Anull%2C%22snap_filter%22%3A%22%22%7D%5D%7D%7D&generalState=%7B%22fileViewOpen%22%3Afalse%2C%22fileViewCollapsed%22%3Atrue%2C%22mainTreeViewCollapsed%22%3Atrue%2C%22callTraceClosed%22%3Afalse%2C%22mainSideNavItem%22%3A%22rules%22%2C%22globalResSelected%22%3Afalse%2C%22isSideBarCollapsed%22%3Afalse%2C%22isRightSideBarCollapsed%22%3Atrue%2C%22selectedFile%22%3A%7B%7D%2C%22fileViewFilter%22%3A%22%22%2C%22mainTreeViewFilter%22%3A%22%22%2C%22contractsFilter%22%3A%22%22%2C%22globalCallResolutionFilter%22%3A%22%22%2C%22currentRuleUiId%22%3A1%2C%22counterExamplePos%22%3A1%2C%22expandedKeysState%22%3A%222-10-1-02-1-1-1-1-1-1-1-1%22%2C%22expandedFilesState%22%3A%5B%5D%2C%22outlinedfilterShared%22%3A%22000000000%22%7D
// in the cex the issuer is supposed to be in the issuers, but assume_pre_has_claim_topic_then_issuer_exists seems
// to be getting a different issuer? 
pub fn remove_claim_topic_integrity_2(e: Env) {
    let claim_topic: u32 = nondet();
    clog!(claim_topic);
    let issuer = nondet_address();
    clog!(cvlr_soroban::Addr(&issuer));
    clog_vec(&get_trusted_issuer_claim_topics(&e, &issuer));
    clog_vec(&get_claim_topics(&e));
    assume_pre_no_duplicate_issuer_topics(e.clone(), issuer.clone()); // invariant TO BE PROVEN BELOW
    assume_pre_has_claim_topic_then_issuer_exists(e.clone(), issuer.clone()); // invariant TO BE PROVEN BELOW
    remove_claim_topic(&e, claim_topic);
    let issuer_claim_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_claim_topics);
    let topics_contains_claim_topic = issuer_claim_topics.contains(claim_topic);
    clog!(topics_contains_claim_topic);
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

#[rule]
// after add_trusted_issuer the issuer exists
// status: verified
pub fn add_trusted_issuer_integrity_1(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    add_trusted_issuer(&e, &issuer, &claim_topics);
    let trusted_issuers_post = get_trusted_issuers(&e);
    let issuer_exists = trusted_issuers_post.contains(&issuer);
    cvlr_assert!(issuer_exists);
}

#[rule]
// after add_trusted_issuer the claim_topics are in the trusted issuer's topics
// status: verified
pub fn add_trusted_issuer_integrity_2(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    let topic: u32 = nondet();
    cvlr_assume!(claim_topics.contains(topic));
    add_trusted_issuer(&e, &issuer, &claim_topics);
    let issuer_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    let issuer_topics_contains_topic = issuer_topics.contains(topic);
    cvlr_assert!(issuer_topics_contains_topic);
}

#[rule]
// after add_trusted_issuer the issuer has_claim_topic for any of the topics
// status: verified
pub fn add_trusted_issuer_integrity_3(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    let topic: u32 = nondet();
    cvlr_assume!(claim_topics.contains(topic));
    add_trusted_issuer(&e, &issuer, &claim_topics);
    let has_claim_topic = has_claim_topic(&e, &issuer, topic);
    cvlr_assert!(has_claim_topic);
}

#[rule]
// after remove_trusted_issuer the issuer does not exist
// status: violation
pub fn remove_trusted_issuer_integrity_1(e: Env) {
    let issuer = nondet_address();
    remove_trusted_issuer(&e, &issuer);
    let trusted_issuers_post = get_trusted_issuers(&e);
    let issuer_exists = trusted_issuers_post.contains(&issuer);
    cvlr_assert!(!issuer_exists);
}

#[rule]
// after remove_trusted_issuer the issuer does not has_claim for any
// status: verified
pub fn remove_trusted_issuer_integrity_2(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    let topic: u32 = nondet();
    cvlr_assume!(claim_topics.contains(topic));
    remove_trusted_issuer(&e, &issuer);
    let has_claim_topic = has_claim_topic(&e, &issuer, topic);
    cvlr_assert!(!has_claim_topic);
}

#[rule]
// after update_issuer_claim_topics the issuer exists
// status: verified
pub fn update_issuer_claim_topics_integrity_1(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    let topic: u32 = nondet();
    cvlr_assume!(claim_topics.contains(topic));
    update_issuer_claim_topics(&e, &issuer, &claim_topics);
    let trusted_issuers_post = get_trusted_issuers(&e);
    let issuer_exists = trusted_issuers_post.contains(&issuer);
    cvlr_assert!(issuer_exists);
}

#[rule]
// after update_issuer_claim_topics the issuer's topics are exactly those given
// status:
pub fn update_issuer_claim_topics_integrity_2(e: Env) {
    let issuer = nondet_address();
    let claim_topics = nondet_vec_u32();
    let topic: u32 = nondet();
    let topic_in_claim_topics = claim_topics.contains(topic);
    update_issuer_claim_topics(&e, &issuer, &claim_topics);
    let issuer_topics: Vec<u32> = get_trusted_issuer_claim_topics(&e, &issuer);
    let topic_in_issuer_topics = issuer_topics.contains(topic);
    cvlr_assert!(topic_in_claim_topics == topic_in_issuer_topics);
}

// invariants

// invariant: topic in get_trusted_issuer_claim_topics(issuer) -> topic in get_claim_topics()

pub fn assume_pre_issuer_topics_subset_all_topics(e: Env, issuer: Address, topic: u32) {
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

// TODO : PROVE

// invariant: no duplicate topics  

pub fn assume_pre_no_duplicate_topics(e: Env) {
    let topics = get_claim_topics(&e);
    clog_vec(&topics);
    let mut seen: Vec<u32> = Vec::new(&e);
    for topic in topics {
        clog!(topic);
        let seen_contains_topic = seen.contains(topic);
        clog!(seen_contains_topic);
        cvlr_assume!(!seen_contains_topic);
        seen.push_back(topic);
    }
}

// TODO : PROVE

// invariant: no duplicate topics for issuer

pub fn assume_pre_no_duplicate_issuer_topics(e: Env, issuer: Address) {
    let topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&topics);
    let mut seen: Vec<u32> = Vec::new(&e);
    for topic in topics {
        clog!(topic);
        let seen_contains_topic = seen.contains(topic);
        clog!(seen_contains_topic);
        cvlr_assume!(!seen_contains_topic);
        seen.push_back(topic);
    }
}

// TODO : PROVE

// invariant: if issuer has_claim_topic then issuer exists

pub fn assume_pre_has_claim_topic_then_issuer_exists(e: Env, issuer: Address) {
    clog!(cvlr_soroban::Addr(&issuer));
    let issuer_topics = get_trusted_issuer_claim_topics(&e, &issuer);
    clog_vec(&issuer_topics);
    let trusted_issuers = get_trusted_issuers(&e);
    clog_vec_addresses(&trusted_issuers);
    let issuer_exists = trusted_issuers.contains(&issuer);
    clog!(issuer_exists);
    clog!(issuer_topics.len());
    if issuer_topics.len() > 0 {
        cvlr_assume!(issuer_exists);
    }
}

// TODO : PROVE

// invariant: if topic has issuer then that issuer has that topic

pub fn assume_pre_has_issuer_then_topic(e: Env, topic: u32, issuer: Address) {
    let issuers_of_topic = get_claim_topic_issuers(&e, topic);
    clog_vec_addresses(&issuers_of_topic);
    let issuer_in_issuers_of_topic = issuers_of_topic.contains(&issuer);
    clog!(issuer_in_issuers_of_topic);
    let issuer_has_topic = has_claim_topic(&e, &issuer, topic);
    clog!(issuer_has_topic);
    if issuer_in_issuers_of_topic {
        cvlr_assume!(issuer_has_topic);
    }
}

// TODO : PROVE

// invariant: if issuer has topic then that topic has that issuer

pub fn assume_pre_has_topic_then_issuer(e: Env, topic: u32, issuer: Address) {
    let issuers_of_topic = get_claim_topic_issuers(&e, topic);
    clog_vec_addresses(&issuers_of_topic);
    let issuer_in_issuers_of_topic = issuers_of_topic.contains(&issuer);
    clog!(issuer_in_issuers_of_topic);
    let issuer_has_topic = has_claim_topic(&e, &issuer, topic);
    clog!(issuer_has_topic);
    if issuer_has_topic {
        cvlr_assume!(issuer_in_issuers_of_topic);
    }
}