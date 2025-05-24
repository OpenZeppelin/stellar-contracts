use soroban_sdk::{Bytes, Env};

/// A trait for hashing an arbitrary stream of bytes.
///
/// Instances of `Hasher` usually represent state that is changed while hashing
/// data.
///
/// `Hasher` provides a fairly basic interface for retrieving the generated hash
/// (with [`Hasher::finalize`]), and absorbing an arbitrary number of bytes
/// (with [`Hasher::update`]). Most of the time, [`Hasher`] instances are used
/// in conjunction with the [`Hashable`] trait.
pub trait Hasher {
    type Output;

    /// Creates a new [`Hasher`] instance.
    fn new(e: &Env) -> Self;

    /// Absorbs additional input. Can be called multiple times.
    fn update(&mut self, input: Bytes);

    /// Outputs the hashing algorithm state.
    fn finalize(self) -> Self::Output;
}
