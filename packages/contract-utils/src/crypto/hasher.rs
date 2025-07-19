use soroban_sdk::{Bytes, Env};

/// A trait for hashing an arbitrary stream of bytes.
///
/// Instances of `Hasher` usually represent state that is changed while hashing
/// data.
///
/// `Hasher` provides a fairly basic interface for retrieving the generated hash
/// (with [`Hasher::finalize`]), and absorbing an arbitrary number of bytes
/// (with [`Hasher::update`]). Most of the time, [`Hasher`] instances are used
/// in conjunction with the [`crate::crypto::hashable::Hashable`] trait.
pub trait Hasher {
    type Output;

    /// Creates a new [`Hasher`] instance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn new(e: &Env) -> Self;

    /// Absorbs additional input. Can be called multiple times.
    ///
    /// # Arguments
    ///
    /// * `input` - Bytes to be added to the internal state.
    fn update(&mut self, input: Bytes);

    /// Outputs the hashing algorithm state.
    ///
    /// # Errors
    ///
    /// * [`crate::crypto::error::CryptoError::HasherEmptyState`] - When the
    ///   state is empty.
    fn finalize(self) -> Self::Output;
}
