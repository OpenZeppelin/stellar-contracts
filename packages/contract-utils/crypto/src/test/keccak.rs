extern crate std;

use std::{format, vec, vec::Vec};

use proptest::prelude::*;
use soroban_sdk::{Bytes, Env};

use crate::{hasher::Hasher, keccak::Keccak256};

fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
}

#[test]
fn single_bit_change_affects_output() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut modified = data.clone();
        modified[0] ^= 1;

        let mut hasher1 = Keccak256::new(&e);
        let mut hasher2 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        hasher2.update(Bytes::from_slice(&e, &modified));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn sequential_updates_match_concatenated() {
    let e = Env::default();
    proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));
        let result1 = hasher1.finalize();

        let mut hasher2 = Keccak256::new(&e);
        let mut concatenated = data1.clone();
        concatenated.extend_from_slice(&data2);
        hasher2.update(Bytes::from_slice(&e, &concatenated));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn split_updates_match_full_update() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy(), split_point: usize)| {
        let split_at = split_point % data.len();

        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data[..split_at]));
        hasher1.update(Bytes::from_slice(&e, &data[split_at..]));
        let result1 = hasher1.finalize();

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn multiple_hasher_instances_are_consistent() {
    let e = Env::default();
    proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));
        let result1 = hasher1.finalize();

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data1));
        hasher2.update(Bytes::from_slice(&e, &data2));
        let result2 = hasher2.finalize();

        prop_assert_eq!(result1.to_array(), result2.to_array());
    })
}

#[test]
fn output_is_always_32_bytes() {
    let e = Env::default();
    proptest!(|(data: Vec<u8>)| {
        let mut hasher = Keccak256::new(&e);
        hasher.update(Bytes::from_slice(&e, &data));
        let result = hasher.finalize();
        assert_eq!(result.to_array().len(), 32);
    })
}

#[test]
fn update_order_dependence() {
    let e = Env::default();
    proptest!(|(data1 in non_empty_u8_vec_strategy(),
                data2 in non_empty_u8_vec_strategy())| {
        prop_assume!(data1 != data2);

        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        hasher1.update(Bytes::from_slice(&e, &data2));

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data2));
        hasher2.update(Bytes::from_slice(&e, &data1));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn empty_input_order_independence() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let empty = vec![];

        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        hasher1.update(Bytes::from_slice(&e, &empty));

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &empty));
        hasher2.update(Bytes::from_slice(&e, &data));

        prop_assert_eq!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn trailing_zero_affects_output() {
    let e = Env::default();
    proptest!(|(data: Vec<u8>)| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));

        let mut padded = data.clone();
        padded.push(0);

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &padded));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn leading_zeros_affect_output() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));
        let hash1 = hasher1.finalize();

        let mut padded = vec![0u8; 32];
        padded.extend(data.iter());

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &padded));
        let hash2 = hasher2.finalize();

        prop_assert_ne!(hash1.to_array(), hash2.to_array());
    })
}

#[test]
fn no_trivial_collisions_same_length() {
    let e = Env::default();
    proptest!(|(data in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data));

        let mut modified = data.clone();
        modified[data.len() - 1] = modified[data.len() - 1].wrapping_add(1);

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &modified));

        prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
    })
}

#[test]
fn length_extension_attack_resistance() {
    let e = Env::default();
    proptest!(|(data1 in non_empty_u8_vec_strategy(), data2 in non_empty_u8_vec_strategy())| {
        let mut hasher1 = Keccak256::new(&e);
        hasher1.update(Bytes::from_slice(&e, &data1));
        let hash1 = hasher1.finalize();

        let mut hasher2 = Keccak256::new(&e);
        hasher2.update(Bytes::from_slice(&e, &data1));
        hasher2.update(Bytes::from_slice(&e, &data2));
        let hash2 = hasher2.finalize();

        let mut hasher3 = Keccak256::new(&e);
        hasher3.update(Bytes::from_slice(&e, &hash1.to_array()));
        hasher3.update(Bytes::from_slice(&e, &data2));
        let hash3 = hasher3.finalize();

        prop_assert_ne!(hash2.to_array(), hash3.to_array());
    })
}

#[test]
fn empty_input() {
    let e = Env::default();
    let mut hasher = Keccak256::new(&e);
    hasher.update(Bytes::from_slice(&e, &[]));
    let result = hasher.finalize();
    let expected: [u8; 32] = [
        0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03,
        0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85,
        0xa4, 0x70,
    ];
    assert_eq!(result.to_array(), expected);
}

#[test]
fn known_hash() {
    let e = Env::default();
    let mut hasher = Keccak256::new(&e);
    hasher.update(Bytes::from_slice(&e, b"hello"));
    let result = hasher.finalize();
    let expected: [u8; 32] = [
        0x1c, 0x8a, 0xff, 0x95, 0x06, 0x85, 0xc2, 0xed, 0x4b, 0xc3, 0x17, 0x4f, 0x34, 0x72, 0x28,
        0x7b, 0x56, 0xd9, 0x51, 0x7b, 0x9c, 0x94, 0x81, 0x27, 0x31, 0x9a, 0x09, 0xa7, 0xa3, 0x6d,
        0xea, 0xc8,
    ];
    assert_eq!(result.to_array(), expected);
}

#[test]
#[should_panic(expected = "Error(Contract, #1402)")]
fn fails_with_emtpy_state() {
    let e = Env::default();
    let hasher = Keccak256::new(&e);
    let _ = hasher.finalize();
}
