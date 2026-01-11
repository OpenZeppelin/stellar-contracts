use cvlr::nondet::{self, Nondet};
use cvlr_soroban::nondet_bytes;
use cvlr::{clog, cvlr_assert, cvlr_satisfy, cvlr_assume};
use cvlr_soroban::{nondet_address};
use crate::crypto::specs::nondet::nondet_keccak;
use crate::crypto::specs::clog::clog_keccak;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Bytes};

use crate::crypto::{hasher::Hasher, keccak::Keccak256};
use crate::crypto::hashable::{Hashable, hash_pair, commutative_hash_pair};

// note: we model keccak and sha256 in the same way, so the verification is only done for keccak

// this rule holds trivialy from our modeling of keccak:

#[rule]
// for a != b the hash of a is different from that of b
// status: verified
pub fn hash_injective(e: &Env) {
    let mut hasher1 = Keccak256::new(e);
    clog_keccak(&hasher1);
    let mut hasher2 = hasher1.clone();
    clog_keccak(&hasher2);
    let a: Bytes = nondet_bytes();
    let b: Bytes = nondet_bytes();
    cvlr_assume!(a != b);
    a.hash(&mut hasher1);
    let hash1 = hasher1.finalize();
    b.hash(&mut hasher2);
    let hash2 = hasher2.finalize();
    clog!(cvlr_soroban::BN(&hash1));
    clog!(cvlr_soroban::BN(&hash2));
    cvlr_assert!(hash1 != hash2);
}


#[rule]
// for a != b the hash of (a,b) is different from that of (b,a)
// status: spurious violation 
// this is due to keccak256 function not being modeled correctly
pub fn hash_pair_non_commutative(e: &Env) {
    // let hasher1: Keccak256 = nondet_keccak(e);
    let hasher1 = Keccak256::new(e); // simple case where starting with empty hasher
    clog_keccak(&hasher1);
    let hasher2 = hasher1.clone();
    clog_keccak(&hasher2);
    let a: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&a));
    let b: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&b));
    cvlr_assume!(a != b);
    let hash1 = hash_pair(&a, &b, hasher1);
    clog!(cvlr_soroban::BN(&hash1));
    let hash2 = hash_pair(&b, &a, hasher2);
    clog!(cvlr_soroban::BN(&hash2));
    cvlr_assert!(hash1 != hash2);
    // cvlr_satisfy!(true);
}

#[rule]
// for arbitrary a,b the commutative_hash_pair of a,b is equal to that of b,a
// status: spurious violation? the data should be equal but it is not
// but it also does not seem like it equals the appended data.
pub fn commutative_hash_pair_equal(e: &Env) {
    let hasher1 = nondet_keccak(e);
    clog_keccak(&hasher1);
    let hasher2 = hasher1.clone();
    clog_keccak(&hasher2);
    let a: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&a));
    let b: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&b));
    let hash1 = commutative_hash_pair(&a, &b, hasher1);
    clog!(cvlr_soroban::BN(&hash1));
    let hash2 = commutative_hash_pair(&b, &a, hasher2);
    clog!(cvlr_soroban::BN(&hash2));
    cvlr_assert!(hash1 == hash2);
}

#[rule]
// hash.update(a).update(b) == hash.update(a.append(b))
// status: spurious violation
// append seems to have weird behavior
// https://prover.certora.com/output/5771024/4eb5b6c2b7164a72885f04402a2ed118/
pub fn update_append_equal(e: &Env) {
    let mut hasher1 = Keccak256::new(e);
    clog_keccak(&hasher1);
    let mut hasher2 = hasher1.clone();
    clog_keccak(&hasher2);
    let a: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&a));
    let b: Bytes = nondet_bytes();
    clog!(cvlr_soroban::B(&b));
    let mut a_copy: Bytes = a.clone();
    clog!(cvlr_soroban::B(&a_copy));
    a_copy.append(&b);
    clog!(cvlr_soroban::B(&a_copy));
    clog!(cvlr_soroban::B(&a));
    hasher1.update(a);
    clog_keccak(&hasher1);
    hasher1.update(b);
    clog_keccak(&hasher1);
    let hash1 = hasher1.finalize();
    clog!(cvlr_soroban::BN(&hash1));
    hasher2.update(a_copy);
    let hash2 = hasher2.finalize();
    clog!(cvlr_soroban::BN(&hash2));
    cvlr_assert!(hash1 == hash2);
}

// TODO: rules about merkle trees
// verify should only succeed if the vector of hashes is exactly
// the right hashes (need to understand what this means)

// e.g.
// proof = (a1,a2)
// leaf = l
// root = r
// leaf = hash(l,a1)
// leaf = hash(hash(l,a1),a2)
// would be accepted only if r=hash(hash(l,a1),a2)

// verify_with_index should only succeed if the proof is exactly
// the right hashes (need to understand what this means)