use soroban_sdk::Env;
use cvlr_soroban_derive::rule;  
use crate::rwa::identity_claims::storage::{ClaimsStorageKey, Claim};
use crate::rwa::identity_claims::storage::{
    add_claim, get_claim, get_claim_ids_by_topic, remove_claim, remove_claim_from_topic_index
};
use soroban_sdk::{BytesN,Vec};
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use crate::rwa::specs::mocks::claim_issuer_trivial::try_is_claim_valid;
use crate::rwa::specs::helpers::clogs::clog_vec_bytes_n;

// helpers

pub fn get_claim_non_pancicking(e: Env, claim_id: BytesN<32>) -> Option<Claim> {
    let key = ClaimsStorageKey::Claim(claim_id.clone());
    e.storage().persistent().get(&key)
}

pub fn get_claim_ids_by_topic_non_pancicking(e: Env, topic: u32) -> Option<Vec<BytesN<32>>> {
    let key = ClaimsStorageKey::ClaimsByTopic(topic);
    e.storage().persistent().get(&key)
}

#[rule]
// after add_claim get_claim does not panic
// status: verified
pub fn add_claim_integrity_1(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    cvlr_assert!(claim_post.is_some());
}

#[rule]
// after add_claim get_claim returns a claim with the same topic
// status: verified
pub fn add_claim_integrity_2(e: Env) {
    let topic: u32 = nondet();
    clog!(topic);
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_topic = claim_post.unwrap().topic;
    clog!(claim_post_topic);
    cvlr_assert!(claim_post_topic == topic);
}

#[rule]
// after add_claim get_claim returns a claim with the same scheme
// status: verified
pub fn add_claim_integrity_3(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    clog!(scheme);
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_scheme = claim_post.unwrap().scheme;
    clog!(claim_post_scheme);
    cvlr_assert!(claim_post_scheme == scheme);
}

#[rule]
// after add_claim get_claim returns a claim with the same issuer
// status: verified
pub fn add_claim_integrity_4(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    clog!(cvlr_soroban::Addr(&issuer));
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_issuer = claim_post.unwrap().issuer;
    clog!(cvlr_soroban::Addr(&claim_post_issuer));
    cvlr_assert!(claim_post_issuer == issuer);
}

#[rule]
// after add_claim get_claim returns a claim with the same signature
// status: verified
pub fn add_claim_integrity_5(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    clog!(cvlr_soroban::B(&signature));
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_signature = claim_post.unwrap().signature;
    clog!(cvlr_soroban::B(&claim_post_signature));
    cvlr_assert!(claim_post_signature == signature);
}

#[rule]
// after add_claim get_claim returns a claim with the same data
// status: verified
pub fn add_claim_integrity_6(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    clog!(cvlr_soroban::B(&data));
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_data = claim_post.unwrap().data;
    clog!(cvlr_soroban::B(&claim_post_data));
    cvlr_assert!(claim_post_data == data);
}

#[rule]
// after add_claim get_claim returns a claim with the same uri
// status: verified
pub fn add_claim_integrity_7(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = add_claim(&e, topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&claim_id));
    let claim_post = get_claim_non_pancicking(e, claim_id);
    let claim_post_uri = claim_post.unwrap().uri;
    cvlr_assert!(claim_post_uri == uri);
}

#[rule]
// after remove_claim, getting the claim returns None
// status: verified
pub fn remove_claim_integrity_1(e: Env) {
    let claim_id = nondet_bytes_n();
    remove_claim(&e, &claim_id);
    let claim_post = get_claim_non_pancicking(e, claim_id);
    cvlr_assert!(claim_post.is_none());
}

#[rule] 
// after remove_claim, the claim is not in get_claim_ids_by_topic()
// status: spurious violation
pub fn remove_claim_integrity_2(e: Env) {
    let claim_id = nondet_bytes_n();
    clog!(cvlr_soroban::BN(&claim_id));
    let topic = nondet();
    clog!(topic);
    remove_claim(&e, &claim_id);
    let claims_by_topic_post = get_claim_ids_by_topic_non_pancicking(e, topic).unwrap();
    clog_vec_bytes_n(&claims_by_topic_post);
    let claims_by_topic_post_contains_claim_id = claims_by_topic_post.contains(&claim_id);
    clog!(claims_by_topic_post_contains_claim_id);
    cvlr_assert!(!claims_by_topic_post_contains_claim_id);
}

// invariants 

// invariant: get_claim doesn't panic -> get_claims_by_topic doesn't panic & claim in claims

pub fn assume_pre_claim_in_claims_by_topic(e: Env, claim_id: BytesN<32>) {
    let claim = get_claim(&e, &claim_id);
    let topic = claim.topic;
    let claims_by_topic_option = get_claim_ids_by_topic_non_pancicking(e, topic);
    cvlr_assume!(claims_by_topic_option.is_some());
    if let Some(claims_by_topic) = claims_by_topic_option {
        let claims_contain_claim_id = claims_by_topic.contains(claim_id.clone());
        cvlr_assume!(claims_contain_claim_id);
    }
}

pub fn assert_post_claim_in_claims_by_topic(e: Env, claim_id: BytesN<32>) {
    let claim = get_claim(&e, &claim_id);
    let topic = claim.topic;
    let claims_by_topic_option = get_claim_ids_by_topic_non_pancicking(e, topic);
    cvlr_assert!(claims_by_topic_option.is_some());
    if let Some(claims_by_topic) = claims_by_topic_option {
        let claims_contain_claim_id = claims_by_topic.contains(claim_id.clone());
        cvlr_assert!(claims_contain_claim_id);
    }
}

// invariant: get_claims_by_topic includes claim_id -> get_claim for claim_id does not panic

pub fn assume_pre_claims_by_topic_then_claim_exists(e: Env, topic: u32, claim_id: BytesN<32>) {
    let claims_by_topic = get_claim_ids_by_topic(&e, topic);
    clog_vec_bytes_n(&claims_by_topic);
    let claims_contain_claim_id = claims_by_topic.contains(claim_id.clone());
    clog!(claims_contain_claim_id);
    if claims_contain_claim_id {
        let claim_option = get_claim_non_pancicking(e, claim_id);
        cvlr_assume!(claim_option.is_some());
    }
}

pub fn assert_post_claims_by_topic_then_claim_exists(e: Env, topic: u32, claim_id: BytesN<32>) {
    let claims_by_topic = get_claim_ids_by_topic(&e, topic);
    clog_vec_bytes_n(&claims_by_topic);
    let claims_contain_claim_id = claims_by_topic.contains(claim_id.clone());
    clog!(claims_contain_claim_id);
    if claims_contain_claim_id {
        let claim_option = get_claim_non_pancicking(e, claim_id);
        cvlr_assert!(claim_option.is_some());
    }
}

// invariant: get_claim -> claim is valid.

pub fn assume_pre_claim_valid(e: Env, claim_id: BytesN<32>) {
    let claim = get_claim(&e, &claim_id);
    clog!(cvlr_soroban::BN(&claim_id));
    let is_valid = try_is_claim_valid(&e, claim.issuer, claim.topic, claim.scheme, claim.signature, claim.data);
    cvlr_assume!(is_valid.unwrap().is_ok());
}

pub fn assert_post_claim_valid(e: Env, claim_id: BytesN<32>) {
    let claim = get_claim(&e, &claim_id);
    clog!(cvlr_soroban::BN(&claim_id));
    let is_valid = try_is_claim_valid(&e, claim.issuer, claim.topic, claim.scheme, claim.signature, claim.data);
    cvlr_assert!(is_valid.unwrap().is_ok());
}

// rules

#[rule]
// status: spurious violation
// in the cex: is_new_claim = false
// so we do not call add_claim_to_topic_index 
// but this means that has(claim_key) is true where claim_key = Claim(claim_id)
// but this is the same claim_id as the one in the inv
// by the invariant in pre this means that get_claims_by_topic contains claim_id
// so does not make sense
pub fn after_add_claim_inv1(e: Env) {
    let topic: u32 = nondet();
    clog!(topic);
    let scheme: u32 = nondet();
    clog!(scheme);
    let issuer = nondet_address();
    clog!(cvlr_soroban::Addr(&issuer));
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = nondet_bytes_n();
    clog!(cvlr_soroban::BN(&claim_id));
    assume_pre_claim_in_claims_by_topic(e.clone(), claim_id.clone());
    let new_id = add_claim(&e.clone(), topic, scheme, &issuer, &signature, &data, &uri);
    clog!(cvlr_soroban::BN(&new_id));
    assert_post_claim_in_claims_by_topic(e, claim_id);
}

#[rule]
// status: verified
pub fn after_add_claim_inv2(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = nondet_bytes_n();
    let topic_inv = nondet();
    assume_pre_claims_by_topic_then_claim_exists(e.clone(), topic_inv, claim_id.clone());
    add_claim(&e.clone(), topic, scheme, &issuer, &signature, &data, &uri);
    assert_post_claims_by_topic_then_claim_exists(e, topic_inv, claim_id);
}

#[rule]
// status: spurious violation (probably, can't clog bytes_n)
pub fn after_add_claim_valid_claims(e: Env) {
    let topic: u32 = nondet();
    let scheme: u32 = nondet();
    let issuer = nondet_address();
    let signature = nondet_bytes();
    let data = nondet_bytes();
    let uri = nondet_string();
    let claim_id = nondet_bytes_n();
    let claim_id_inv = nondet_bytes_n();
    assume_pre_claim_valid(e.clone(), claim_id_inv.clone());
    add_claim(&e.clone(), topic, scheme, &issuer, &signature, &data, &uri);
    assert_post_claim_valid(e, claim_id_inv);
}

#[rule]
// status: verified
pub fn after_remove_claim_inv1(e: Env) {
    let claim_id = nondet_bytes_n();
    let claim_id_inv = nondet_bytes_n();
    assume_pre_claim_in_claims_by_topic(e.clone(), claim_id_inv.clone());
    remove_claim(&e.clone(), &claim_id);
    assert_post_claim_in_claims_by_topic(e, claim_id_inv);
}

#[rule]
// status: spurious violation
pub fn after_remove_claim_inv2(e: Env) {
    let claim_id = nondet_bytes_n();
    let topic_inv = nondet();
    let claim_id_inv = nondet_bytes_n();
    assume_pre_claims_by_topic_then_claim_exists(e.clone(), topic_inv, claim_id_inv.clone());
    remove_claim(&e.clone(), &claim_id);
    assert_post_claims_by_topic_then_claim_exists(e, topic_inv, claim_id_inv);
}

#[rule]
// status: spurious violation (probably, can't clog bytes_n)
pub fn after_remove_claim_valid_claims(e: Env) {
    let claim_id = nondet_bytes_n();
    let claim_id_inv = nondet_bytes_n();
    assume_pre_claim_valid(e.clone(), claim_id_inv.clone());
    remove_claim(&e.clone(), &claim_id);
    assert_post_claim_valid(e, claim_id_inv);
}