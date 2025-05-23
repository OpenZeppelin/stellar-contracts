use soroban_sdk::Env;

use crate::hashable::Hashable;

/// A trait for hashing an arbitrary stream of bytes.
///
/// Instances of `Hasher` usually represent state that is changed while hashing
/// data.
///
/// `Hasher` provides a fairly basic interface for retrieving the generated hash
/// (with [`Hasher::finalize`]), and absorbing an arbitrary number of bytes
/// (with [`Hasher::update`]). Most of the time, [`Hasher`] instances are used
/// in conjunction with the [`Hashable`] trait.
pub trait Hasher: Sized {
    type Output;

    /// Creates a new [`Hasher`] instance.
    fn new(e: &Env) -> Self;

    /// Absorbs additional input. Can be called multiple times.
    fn update(&mut self, input: impl AsRef<[u8]>);

    /// Outputs the hashing algorithm state.
    fn finalize(self) -> Self::Output;

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
    fn hash_one<T>(e: &Env, h: T) -> Self::Output
    where
        T: Hashable,
    {
        let mut hasher: Self = Hasher::new(e);
        h.hash(&mut hasher);
        hasher.finalize()
    }
}
