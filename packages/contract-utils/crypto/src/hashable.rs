//! Generic hashing support.

use soroban_sdk::BytesN;

use crate::hasher::Hasher;

/// A hashable type.
///
/// Types implementing `Hash` are able to be [`Hash::hash`]ed with an instance
/// of [`Hasher`].
pub trait Hashable {
    /// Feeds this value into the given [`Hasher`].
    fn hash<H: Hasher>(&self, state: &mut H);
}

impl Hashable for BytesN<32> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.update(self.to_array());
    }
}

/// Hash the pair `(a, b)` with `state`.
///
/// Returns the finalized hash output from the hasher.
///
/// # Arguments
///
/// * `a` - The first value to hash.
/// * `b` - The second value to hash.
/// * `state` - The hasher state to use.
#[inline]
pub fn hash_pair<S, H>(a: &H, b: &H, mut state: S) -> S::Output
where
    H: Hashable + ?Sized,
    S: Hasher,
{
    a.hash(&mut state);
    b.hash(&mut state);
    state.finalize()
}

/// Sort the pair `(a, b)` and hash the result with `state`. Frequently used
/// when working with merkle proofs.
#[inline]
pub fn commutative_hash_pair<S, H>(a: &H, b: &H, state: S) -> S::Output
where
    H: Hashable + PartialOrd,
    S: Hasher,
{
    if a > b {
        hash_pair(b, a, state)
    } else {
        hash_pair(a, b, state)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::{format, vec::Vec};

    use proptest::prelude::*;
    use soroban_sdk::Env;

    use super::*;
    use crate::{hasher::BuildHasher, keccak::KeccakBuilder};

    // Helper impl for testing
    impl Hashable for Vec<u8> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            state.update(self.as_slice());
        }
    }

    fn non_empty_u8_vec_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..ProptestConfig::default().max_default_size_range)
    }

    #[test]
    fn commutative_hash_is_order_independent() {
        let e = Env::default();
        proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);
            let hash1 = commutative_hash_pair(&a, &b, builder.build_hasher());
            let hash2 = commutative_hash_pair(&b, &a, builder.build_hasher());
            prop_assert_eq!(hash1, hash2);
        })
    }

    #[test]
    fn regular_hash_is_order_dependent() {
        let e = Env::default();
        proptest!(|(a in non_empty_u8_vec_strategy(),
        b in non_empty_u8_vec_strategy())| {
            prop_assume!(a != b);
            let builder = KeccakBuilder::new(&e);
            let hash1 = hash_pair(&a, &b, builder.build_hasher());
            let hash2 = hash_pair(&b, &a, builder.build_hasher());
            prop_assert_ne!(hash1, hash2);
        })
    }

    #[test]
    fn hash_pair_deterministic() {
        let e = Env::default();
        proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);
            let hash1 = hash_pair(&a, &b, builder.build_hasher());
            let hash2 = hash_pair(&a, &b, builder.build_hasher());
            prop_assert_eq!(hash1, hash2);
        })
    }

    #[test]
    fn commutative_hash_pair_deterministic() {
        let e = Env::default();
        proptest!(|(a: Vec<u8>, b: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);
            let hash1 = commutative_hash_pair(&a, &b, builder.build_hasher());
            let hash2 = commutative_hash_pair(&a, &b, builder.build_hasher());
            prop_assert_eq!(hash1, hash2);
        })
    }

    #[test]
    fn identical_pairs_hash() {
        let e = Env::default();
        proptest!(|(a: Vec<u8>)| {
            let builder = KeccakBuilder::new(&e);
            let hash1 = hash_pair(&a, &a, builder.build_hasher());
            let hash2 = commutative_hash_pair(&a, &a, builder.build_hasher());
            assert_eq!(hash1, hash2);
        })
    }
}
