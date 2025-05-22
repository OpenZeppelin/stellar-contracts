use soroban_sdk::{Bytes, BytesN, Env};

use crate::hasher::{Hasher, HasherBuilder};

pub type KeccakBuilder = HasherBuilder<Keccak256>;

pub struct Keccak256 {
    buffer: Option<Bytes>,
    env: Env,
}

impl Hasher for Keccak256 {
    type Output = BytesN<32>;

    fn new(e: &Env) -> Self {
        Keccak256 { buffer: None, env: e.clone() }
    }

    fn update(&mut self, input: impl AsRef<[u8]>) {
        let bytes = Bytes::from_slice(&self.env, input.as_ref());
        match &mut self.buffer {
            None => self.buffer = Some(bytes),
            Some(buffer) => buffer.append(&bytes),
        }
    }

    fn finalize(self) -> Self::Output {
        let data = self.buffer.expect("No data to hash: buffer empty!");
        self.env.crypto().keccak256(&data).to_bytes()
    }
}

#[cfg(test)]
mod test {

    extern crate std;

    use std::{format, vec, vec::Vec};

    use proptest::prelude::*;

    use super::*;
    use crate::hasher::BuildHasher;

    fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
    }

    #[test]
    fn single_bit_change_affects_output() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut modified = data.clone();
            modified[0] ^= 1;

            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data);
            hasher2.update(&modified);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn sequential_updates_match_concatenated() {
        let e = Env::default();
        proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);

            let mut hasher1 = builder.build_hasher();
            hasher1.update(&data1);
            hasher1.update(&data2);
            let result1 = hasher1.finalize();

            let mut hasher2 = builder.build_hasher();
            let mut concatenated = data1.clone();
            concatenated.extend_from_slice(&data2);
            hasher2.update(concatenated);
            let result2 = hasher2.finalize();

            prop_assert_eq!(result1.to_array(), result2.to_array());
        })
    }

    #[test]
    fn split_updates_match_full_update() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy(), split_point: usize)| {
            let builder = KeccakBuilder::new(&e);
            let split_at = split_point % data.len();

            let mut hasher1 = builder.build_hasher();
            hasher1.update(&data[..split_at]);
            hasher1.update(&data[split_at..]);
            let result1 = hasher1.finalize();

            let mut hasher2 = builder.build_hasher();
            hasher2.update(&data);
            let result2 = hasher2.finalize();

            prop_assert_eq!(result1.to_array(), result2.to_array());
        })
    }

    #[test]
    fn multiple_hasher_instances_are_consistent() {
        let e = Env::default();
        proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);

            let mut hasher1 = builder.build_hasher();
            hasher1.update(&data1);
            hasher1.update(&data2);
            let result1 = hasher1.finalize();

            let mut hasher2 = builder.build_hasher();
            hasher2.update(&data1);
            hasher2.update(&data2);
            let result2 = hasher2.finalize();

            prop_assert_eq!(result1.to_array(), result2.to_array());
        })
    }

    #[test]
    fn output_is_always_32_bytes() {
        let e = Env::default();
        proptest!(|(data: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);
            let mut hasher = builder.build_hasher();
            hasher.update(&data);
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

            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data1);
            hasher1.update(&data2);

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&data2);
            hasher2.update(&data1);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn empty_input_order_independence() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let empty = vec![];

            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data);
            hasher1.update(&empty);

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&empty);
            hasher2.update(&data);

            prop_assert_eq!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn trailing_zero_affects_output() {
        let e = Env::default();
        proptest!(|(data: Vec<u8>)| {
            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data);

            let mut padded = data.clone();
            padded.push(0);

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&padded);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn leading_zeros_affect_output() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data);
            let hash1 = hasher1.finalize();

            let mut padded = vec![0u8; 32];
            padded.extend(data.iter());

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&padded);
            let hash2 = hasher2.finalize();

            prop_assert_ne!(hash1.to_array(), hash2.to_array());
        })
    }

    #[test]
    fn no_trivial_collisions_same_length() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data);

            let mut modified = data.clone();
            modified[data.len() - 1] = modified[data.len() - 1].wrapping_add(1);

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&modified);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn length_extension_attack_resistance() {
        let e = Env::default();
        proptest!(|(data1 in non_empty_u8_vec_strategy(), data2 in non_empty_u8_vec_strategy())| {
            let mut hasher1 = KeccakBuilder::new(&e).build_hasher();
            hasher1.update(&data1);
            let hash1 = hasher1.finalize();

            let mut hasher2 = KeccakBuilder::new(&e).build_hasher();
            hasher2.update(&data1);
            hasher2.update(&data2);
            let hash2 = hasher2.finalize();

            let mut hasher3 = KeccakBuilder::new(&e).build_hasher();
            hasher3.update(hash1.to_array());
            hasher3.update(&data2);
            let hash3 = hasher3.finalize();

            prop_assert_ne!(hash2.to_array(), hash3.to_array());
        })
    }

    #[test]
    fn empty_input() {
        let e = Env::default();
        let builder = KeccakBuilder::new(&e);
        let result = builder.hash_one(vec![]);
        let expected: [u8; 32] = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(result.to_array(), expected);
    }

    #[test]
    fn known_hash() {
        let e = Env::default();
        let builder = KeccakBuilder::new(&e);
        let mut hasher = builder.build_hasher();
        hasher.update(b"hello");
        let result = hasher.finalize();
        let expected: [u8; 32] = [
            0x1c, 0x8a, 0xff, 0x95, 0x06, 0x85, 0xc2, 0xed, 0x4b, 0xc3, 0x17, 0x4f, 0x34, 0x72,
            0x28, 0x7b, 0x56, 0xd9, 0x51, 0x7b, 0x9c, 0x94, 0x81, 0x27, 0x31, 0x9a, 0x09, 0xa7,
            0xa3, 0x6d, 0xea, 0xc8,
        ];
        assert_eq!(result.to_array(), expected);
    }
}
