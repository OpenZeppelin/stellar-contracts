extern crate std;

use std::{format, vec, vec::Vec};

use proptest::prelude::*;
use soroban_sdk::{Bytes, Env};

use crate::{hasher::Hasher, sha256::Sha256};

fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
}

#[test]
fn sha256_single_bit_change_affects_output() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut modified = data.clone();
        modified[0] ^= 1;

        let mut hasher1 = Sha256::new(&e);
        let mut hasher2 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        hasher2.update(Bytes::from_slice(&e, &modified));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sha256_sequential_updates_match_concatenated() {
    let e = Env::default();
    proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));
        let result1 = hasher1.finalize();

        let mut hasher2 = Sha256::new(&e);
        let mut concatenated = data1.clone();
        concatenated.extend_from_slice(&data2);
        hasher2.update(Bytes::from_slice(&e, &concatenated));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn sha256_split_updates_match_full_update() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy(), split_point: usize)| {
        let split_at = split_point % data.len();

        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data[..split_at]));
        hasher1.update(Bytes::from_slice(&e, &data[split_at..]));
        let result1 = hasher1.finalize();

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn sha256_multiple_hasher_instances_are_consistent() {
    let e = Env::default();
    proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));
        let result1 = hasher1.finalize();

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data1));
        hasher2.update(Bytes::from_slice(&e, &data2));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn sha256_output_is_always_32_bytes() {
    let e = Env::default();
    proptest!(|(data: Vec<u8>)| {
        let mut hasher = Sha256::new(&e);
        hasher.update(Bytes::from_slice(&e, &data));
        let result = hasher.finalize();
        assert_eq!(result.to_array().len(), 32);
    })
}

#[test]
fn sha256_update_order_dependence() {
    let e = Env::default();
    proptest!(|(data1 in non_empty_u8_vec_strategy(),
                data2 in non_empty_u8_vec_strategy())| {
        prop_assume!(data1 != data2);

        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data2));
        hasher2.update(Bytes::from_slice(&e, &data1));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sha256_empty_input_order_independence() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let empty = vec![];

        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        hasher1.update(Bytes::from_slice(&e, &empty));

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &empty));
        hasher2.update(Bytes::from_slice(&e, &data));

        prop_assert_eq!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sha256_trailing_zero_affects_output() {
    let e = Env::default();
    proptest!(|(data: Vec<u8>)| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));

        let mut padded = data.clone();
        padded.push(0);

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &padded));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sha256_leading_zeros_affect_output() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        let hash1 = hasher1.finalize();

        let mut padded = vec![0u8; 32];
        padded.extend(data.iter());

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &padded));
        let hash2 = hasher2.finalize();

        prop_assert_ne!(hash1.to_array(), hash2.to_array());
    })
}

#[test]
fn sha256_no_trivial_collisions_same_length() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));

        let mut modified = data.clone();
        modified[data.len() - 1] = modified[data.len() - 1].wrapping_add(1);

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &modified));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sha256_length_extension_attack_resistance() {
    let e = Env::default();
    proptest!(|(data1 in non_empty_u8_vec_strategy(), data2 in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Sha256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        let hash1 = hasher1.finalize();

        let mut hasher2 = Sha256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data1));
        hasher2.update(Bytes::from_slice(&e, &data2));
        let hash2 = hasher2.finalize();

        let mut hasher3 = Sha256::new(&e);
        hasher3.update(Bytes::from_slice(&e, &hash1.to_array()));
        hasher3.update(Bytes::from_slice(&e, &data2));
        let hash3 = hasher3.finalize();

        prop_assert_ne!(hash2.to_array(), hash3.to_array());
    })
}

#[test]
fn sha256_empty_input() {
    let e = Env::default();
    let mut hasher = Sha256::new(&e);
    hasher.update(Bytes::from_slice(&e, &[]));
    let result = hasher.finalize();
    let expected: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    assert_eq!(result.to_array(), expected);
}

#[test]
fn sha256_known_hash() {
    let e = Env::default();
    let mut hasher = Sha256::new(&e);
    hasher.update(Bytes::from_slice(&e, b"hello"));
    let result = hasher.finalize();
    let expected: [u8; 32] = [
        0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9, 0xe2,
        0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62, 0x93, 0x8b,
        0x98, 0x24,
    ];
    assert_eq!(result.to_array(), expected);
}

#[test]
#[should_panic(expected = "Error(Contract, #1402)")]
fn sha256_fails_with_empty_state() {
    let e = Env::default();
    let hasher = Sha256::new(&e);
    let _ = hasher.finalize();
}
