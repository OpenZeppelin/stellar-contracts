use core::marker::PhantomData;

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
/// in conjunction with the [`Hash`] trait.
pub trait Hasher {
    type Output;

    fn new(e: &Env) -> Self;

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
pub trait BuildHasher<H: Hasher> {
    fn new(e: &Env) -> Self;
    /// Creates a new hasher.
    ///
    /// Each call to `build_hasher` on the same instance should produce
    /// identical [`Hasher`]s.
    fn build_hasher(&self) -> H;

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
    fn hash_one<T>(&self, h: T) -> H::Output
    where
        T: Hashable,
    {
        let mut hasher = self.build_hasher();
        h.hash(&mut hasher);
        hasher.finalize()
    }
}

pub struct HasherBuilder<H: Hasher> {
    env: Env,
    _marker: PhantomData<H>,
}

impl<H: Hasher> BuildHasher<H> for HasherBuilder<H> {
    fn new(e: &Env) -> Self {
        Self { env: e.clone(), _marker: PhantomData }
    }

    fn build_hasher(&self) -> H {
        H::new(&self.env)
    }
}
