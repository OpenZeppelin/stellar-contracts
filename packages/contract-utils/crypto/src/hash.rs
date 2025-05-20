//! Generic hashing support.

/// A hashable type.
///
/// Types implementing `Hash` are able to be [`Hash::hash`]ed with an instance
/// of [`Hasher`].
pub trait Hashable {
    /// Feeds this value into the given [`Hasher`].
    fn hash<H: Hasher>(&self, state: &mut H);
}

/// A trait for hashing an arbitrary stream of bytes.
///
/// Instances of `Hasher` usually represent state that is changed while hashing
/// data.
///
/// `Hasher` provides a fairly basic interface for retrieving the generated hash
/// (with [`Hasher::finalize`]), and absorbing an arbitrary number of bytes
/// (with [`Hasher::update`]). Most of the time, [`Hasher`] instances are used
/// in conjunction with the [`Hash`] trait.
pub trait Hasher {
    /// The output type of this hasher.
    type Output;

    /// Absorb additional input. Can be called multiple times.
    fn update(&mut self, input: impl AsRef<[u8]>);

    /// Output the hashing algorithm state.
    fn finalize(self) -> Self::Output;
}

/// A trait for creating instances of [`Hasher`].
///
/// A `BuildHasher` is typically used (e.g., by [`HashMap`]) to create
/// [`Hasher`]s for each key such that they are hashed independently of one
/// another, since [`Hasher`]s contain state.
///
/// For each instance of `BuildHasher`, the [`Hasher`]s created by
/// [`build_hasher`] should be identical. That is, if the same stream of bytes
/// is fed into each hasher, the same output will also be generated.
pub trait BuildHasher {
    /// Type of the hasher that will be created.
    type Hasher: Hasher;

    /// Creates a new hasher.
    ///
    /// Each call to `build_hasher` on the same instance should produce
    /// identical [`Hasher`]s.
    fn build_hasher(&self) -> Self::Hasher;

    /// Calculates the hash of a single value.
    ///
    /// This is intended as a convenience for code which *consumes* hashes, such
    /// as the implementation of a hash table or in unit tests that check
    /// whether a custom [`Hashable`] implementation behaves as expected.
    ///
    /// This must not be used in any code which *creates* hashes, such as in an
    /// implementation of [`Hashable`].  The way to create a combined hash of
    /// multiple values is to call [`Hashable::hash`] multiple times using the
    /// same [`Hasher`], not to call this method repeatedly and combine the
    /// results.
    fn hash_one<H>(&self, h: H) -> <Self::Hasher as Hasher>::Output
    where
        H: Hashable,
        Self::Hasher: Hasher,
    {
        let mut hasher = self.build_hasher();
        h.hash(&mut hasher);
        hasher.finalize()
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
    use crate::keccak::KeccakBuilder;

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
