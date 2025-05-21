use soroban_sdk::{Bytes, BytesN, Env};

use crate::hashable::{BuildHasher, Hasher};

pub struct Sha256Builder {
    env: Env,
}

impl Sha256Builder {
    pub fn new(e: &Env) -> Self {
        Sha256Builder { env: e.clone() }
    }
}

impl BuildHasher for Sha256Builder {
    type Hasher = Sha256;

    fn build_hasher(&self) -> Self::Hasher {
        Sha256 { buffer: None, env: self.env.clone() }
    }
}

pub struct Sha256 {
    buffer: Option<Bytes>,
    env: Env,
}

impl Hasher for Sha256 {
    type Output = BytesN<32>;

    fn update(&mut self, input: impl AsRef<[u8]>) {
        let bytes = Bytes::from_slice(&self.env, input.as_ref());
        match &mut self.buffer {
            None => self.buffer = Some(bytes),
            Some(buffer) => buffer.append(&bytes),
        }
    }

    fn finalize(self) -> Self::Output {
        match &self.buffer {
            // panic ??
            None => unimplemented!(),
            Some(b) => self.env.crypto().sha256(b).to_bytes(),
        }
    }
}

#[cfg(test)]
mod test {

    extern crate std;

    use std::{format, vec, vec::Vec};

    use proptest::prelude::*;

    use super::*;

    fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
    }

    #[test]
    fn single_bit_change_affects_output() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut modified = data.clone();
            modified[0] ^= 1;

            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data);
            hasher2.update(&modified);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn sequential_updates_match_concatenated() {
        let e = Env::default();
        proptest!(|(data1: Vec<u8>, data2: Vec<u8>)| {
            let builder = Sha256Builder::new(&e);

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
            let builder = Sha256Builder::new(&e);
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
            let builder = Sha256Builder::new(&e);

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
            let builder = Sha256Builder::new(&e);
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

            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data1);
            hasher1.update(&data2);

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
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

            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data);
            hasher1.update(&empty);

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher2.update(&empty);
            hasher2.update(&data);

            prop_assert_eq!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn trailing_zero_affects_output() {
        let e = Env::default();
        proptest!(|(data: Vec<u8>)| {
            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data);

            let mut padded = data.clone();
            padded.push(0);

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher2.update(&padded);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn leading_zeros_affect_output() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data);
            let hash1 = hasher1.finalize();

            let mut padded = vec![0u8; 32];
            padded.extend(data.iter());

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher2.update(&padded);
            let hash2 = hasher2.finalize();

            prop_assert_ne!(hash1.to_array(), hash2.to_array());
        })
    }

    #[test]
    fn no_trivial_collisions_same_length() {
        let e = Env::default();
        proptest!(|(data in non_empty_u8_vec_strategy())| {
            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data);

            let mut modified = data.clone();
            modified[data.len() - 1] = modified[data.len() - 1].wrapping_add(1);

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher2.update(&modified);

            prop_assert_ne!(hasher1.finalize().to_array(), hasher2.finalize().to_array());
        })
    }

    #[test]
    fn length_extension_attack_resistance() {
        let e = Env::default();
        proptest!(|(data1 in non_empty_u8_vec_strategy(), data2 in non_empty_u8_vec_strategy())| {
            let mut hasher1 = Sha256Builder::new(&e).build_hasher();
            hasher1.update(&data1);
            let hash1 = hasher1.finalize();

            let mut hasher2 = Sha256Builder::new(&e).build_hasher();
            hasher2.update(&data1);
            hasher2.update(&data2);
            let hash2 = hasher2.finalize();

            let mut hasher3 = Sha256Builder::new(&e).build_hasher();
            hasher3.update(hash1.to_array());
            hasher3.update(&data2);
            let hash3 = hasher3.finalize();

            prop_assert_ne!(hash2.to_array(), hash3.to_array());
        })
    }

    #[test]
    fn sha256_empty_input() {
        let e = Env::default();
        let builder = Sha256Builder::new(&e);
        let mut hasher = builder.build_hasher();
        hasher.update([]);
        let result = hasher.finalize();
        let expected: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(result.to_array(), expected);
    }

    #[test]
    fn sha256_known_hash() {
        let e = Env::default();
        let builder = Sha256Builder::new(&e);
        let mut hasher = builder.build_hasher();
        hasher.update(b"hello");
        let result = hasher.finalize();
        let expected: [u8; 32] = [
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9,
            0xe2, 0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62,
            0x93, 0x8b, 0x98, 0x24,
        ];
        assert_eq!(result.to_array(), expected);
    }
}
