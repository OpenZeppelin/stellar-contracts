//! Generic hashing support.

use soroban_sdk::{Bytes, BytesN};

use crate::crypto::hasher::Hasher;

/// A hashable type.
///
/// Types implementing `Hashable` are able to be [`Hashable::hash`]ed with an
/// instance of [`Hasher`].
pub trait Hashable {
    /// Feeds this value into the given [`Hasher`].
    fn hash<H: Hasher>(&self, hasher: &mut H);
}

impl Hashable for BytesN<32> {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.update(self.into());
    }
}

impl Hashable for Bytes {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.update(self.clone());
    }
}

/// Hash the pair `(a, b)` with `hasher`.
///
/// Returns the finalized hash output from the hasher.
///
/// # Arguments
///
/// * `a` - The first value to hash.
/// * `b` - The second value to hash.
/// * `hasher` - The hasher to use.
#[inline]
pub fn hash_pair<S, H>(a: &H, b: &H, mut hasher: S) -> S::Output
where
    H: Hashable + ?Sized,
    S: Hasher,
{
    a.hash(&mut hasher);
    b.hash(&mut hasher);
    hasher.finalize()
}

/// Sort the pair `(a, b)` and hash the result with `hasher`. Frequently used
/// when working with merkle proofs.
///
/// Returns the finalized hash output from the hasher.
///
/// # Arguments
///
/// * `a` - The first value to hash.
/// * `b` - The second value to hash.
/// * `hasher` - The hasher to use.
#[inline]
pub fn commutative_hash_pair<S, H>(a: &H, b: &H, hasher: S) -> S::Output
where
    H: Hashable + PartialOrd,
    S: Hasher,
{
    if a > b {
        hash_pair(b, a, hasher)
    } else {
        hash_pair(a, b, hasher)
    }
}
