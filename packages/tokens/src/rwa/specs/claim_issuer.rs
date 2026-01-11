// netanel's ideas
// Invariants:
//If ClaimIssuerStorageKey::Pairs(SigningKey) exist, then it points to a non-empty vector.
//A key which is part of a vector of ClaimIssuerStorageKey:: Topics(u32) must have at least one associated registry for that topic
// (i.e., if the topic is 25 the value of  ClaimIssuerStorageKey::Pairs(SigningKey) is Vec<(Topic, Registry) and we want Vec<(25, Registry) to be non-empty).
// roperties:
// The data structures SigningKey (tracks the topic-registry pairs for which a given signing key is authorized) and ClaimIssuerStorageKey (tracks which signing keys
// are authorized to sign claims for a specific topic) are correctly correlated;

// claim_issuer based on claim_topics_and_issuers

// view functions
// get_keys_for_topic
// get_registries
// is_key_allowed_for_topic
// is_key_allowed_for_registry
// is_authorized_for

// functions
// allow_key
// remove_key
// set_claim_revoked

// other:
// get_current_nonce_for
// invalidate_claim_signatures
// is_claim_revoked
// is_claim_expired
// encode_claim_data_expiration
// decode_claim_data_expiration
// build_claim_message
// build_claim_identifier
// extract_from_bytes

use soroban_sdk::{Env, Address, Vec};
use cvlr_soroban_derive::rule;  
use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string};
use crate::rwa::claim_issuer::storage::{allow_key, get_keys_for_topic, SigningKey};
use crate::rwa::claim_issuer::ClaimIssuer;
use crate::rwa::specs::helpers::clogs::{clog_vec, clog_vec_signing_keys};

#[rule]
// after allow_key the keys for the topic contains the signging key of public_key+scheme
// status: spurious violaiton
// seems to be the same kind of spurious violation where .contains is weird.
// is_key_allowed_for_topic returns true so the topic is not added
pub fn allow_key_integrity_1(e: Env) {
    let public_key = nondet_bytes();
    clog!(cvlr_soroban::B(&public_key));
    let registry = nondet_address();
    clog!(cvlr_soroban::Addr(&registry));
    let scheme = nondet();
    clog!(scheme);
    let claim_topic = nondet();
    clog!(claim_topic);
    let keys_for_topic_pre = get_keys_for_topic(&e, claim_topic);
    clog_vec_signing_keys(&keys_for_topic_pre);
    allow_key(&e, &public_key, &registry, scheme, claim_topic);
    let keys_for_topic_post = get_keys_for_topic(&e, claim_topic);
    clog_vec_signing_keys(&keys_for_topic_post);
    let expected_siging_key = SigningKey { public_key: public_key.clone(), scheme };
    let keys_post_contains_expected_key = keys_for_topic_post.contains(&expected_siging_key);
    cvlr_assert!(keys_post_contains_expected_key);
}

// after allow_key is_key_allowed_for_topic for the key+claim_topic returns true 

// after allow_key is_key_allowed_for_registry for the key+registry returns true

// after allow_key is_authorized_for for the registry+claim_topic returns true

// after allow_key get_registries(key) contains the registry

// 