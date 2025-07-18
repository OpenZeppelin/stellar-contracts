extern crate std;

use std::{format, vec::Vec};

use proptest::prelude::*;
use soroban_sdk::{Bytes, Env};

use crate::crypto::{hashable::*, hasher::Hasher, keccak::Keccak256};

fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
}

#[test]
fn commutative_hash_is_order_independent() {
    let e = Env::default();
    proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
        let a = Bytes::from_slice(&e, &a);
        let b = Bytes::from_slice(&e, &b);
        let hash1 = commutative_hash_pair(&a, &b, Keccak256::new(&e));
        let hash2 = commutative_hash_pair(&b, &a, Keccak256::new(&e));
        prop_assert_eq!(hash1, hash2);
    })
}

#[test]
fn regular_hash_is_order_dependent() {
    let e = Env::default();
    proptest!(|(a in non_empty_u8_vec_strategy(),
    b in non_empty_u8_vec_strategy())| {
        prop_assume!(a != b);
        let a = Bytes::from_slice(&e, &a);
        let b = Bytes::from_slice(&e, &b);
        let hash1 = hash_pair(&a, &b, Keccak256::new(&e));
        let hash2 = hash_pair(&b, &a, Keccak256::new(&e));
        prop_assert_ne!(hash1, hash2);
    })
}

#[test]
fn hash_pair_deterministic() {
    let e = Env::default();
    proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
        let a = Bytes::from_slice(&e, &a);
        let b = Bytes::from_slice(&e, &b);
        let hash1 = hash_pair(&a, &b, Keccak256::new(&e));
        let hash2 = hash_pair(&a, &b, Keccak256::new(&e));
        prop_assert_eq!(hash1, hash2);
    })
}

#[test]
fn commutative_hash_pair_deterministic() {
    let e = Env::default();
    proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
        let a = Bytes::from_slice(&e, &a);
        let b = Bytes::from_slice(&e, &b);
        let hash1 = commutative_hash_pair(&a, &b, Keccak256::new(&e));
        let hash2 = commutative_hash_pair(&a, &b, Keccak256::new(&e));
        prop_assert_eq!(hash1, hash2);
    })
}

#[test]
fn identical_pairs_hash() {
    let e = Env::default();
    proptest!(|(a: Vec<u8>)| {
        let a = Bytes::from_slice(&e, &a);
        let hash1 = hash_pair(&a, &a, Keccak256::new(&e));
        let hash2 = commutative_hash_pair(&a, &a, Keccak256::new(&e));
        assert_eq!(hash1, hash2);
    })
}
