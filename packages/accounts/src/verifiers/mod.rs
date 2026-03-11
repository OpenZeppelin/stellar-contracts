//! # Verifier Building Blocks
//!
//! This module contains the core `Verifier` trait and functions necessary to
//! implement cryptographic signature verification for smart accounts. It
//! provides utility functions for `ed25519` signature verification and
//! `webauthn` (passkey authentication) that can be used to build verifier
//! contracts.
pub mod ed25519;
#[cfg(test)]
mod test;
pub mod utils;
pub mod webauthn;
use soroban_sdk::{contractclient, Bytes, Env, FromVal, Val, Vec};

/// Core trait for cryptographic signature verification in smart accounts.
///
/// This trait defines the interface for verifying digital signatures against
/// cryptographic keys. Implementations handle different signature schemes
/// (e.g., Ed25519, WebAuthn) and provide a unified interface meant to be used
/// in smart accounts.
///
/// # Type Parameters
///
/// * `SigData` - The signature data type specific to the verification scheme.
///   Must implement `FromVal<Env, Val>`.
/// * `KeyData` - The public key type specific to the verification scheme. Must
///   implement `FromVal<Env, Val>`.
///
/// # Implementation Notes
///
/// Verifiers should:
/// - Validate input parameters (hash, key_data, sig_data) for correctness
/// - Perform cryptographic verification according to their specific scheme
/// - Panic with appropriate errors for malformed inputs
///
/// # Examples
///
/// ```rust
/// use soroban_sdk::{Bytes, Env};
/// use stellar_accounts::verifiers::Verifier;
///
/// struct MyVerifier;
/// impl Verifier for MyVerifier {
///     type KeyData = BytesN<32>;
///     type SigData = BytesN<64>;
///
///     fn verify(e: &Env, hash: Bytes, key_data: BytesN<32>, sig_data: BytesN<64>) -> bool {
///         // Implementation specific verification logic
///         true
///     }
/// }
/// ```
pub trait Verifier {
    type KeyData: FromVal<Env, Val>;
    type SigData: FromVal<Env, Val>;

    /// Verifies a cryptographic signature against a hash and public key.
    ///
    /// This method performs cryptographic verification of a digital signature
    /// according to the specific signature scheme implemented by the verifier.
    /// It validates that the signature was created by the holder of the private
    /// key corresponding to the provided public key data.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `hash` - The hash of the data that was signed (typically 32 bytes).
    /// * `key_data` - The public key data in the format expected by this
    ///   verifier.
    /// * `sig_data` - The signature data in the format expected by this
    ///   verifier.
    ///
    /// # Returns
    ///
    /// `true` if the signature is valid and was created by the private key
    /// corresponding to `key_data`, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Implementations should panic with appropriate error codes when:
    /// - `key_data` is malformed or has invalid length
    /// - `sig_data` is malformed or has invalid format
    /// - `hash` has invalid length for the signature scheme
    /// - Other cryptographic validation failures occur
    ///
    /// # Security Requirements
    ///
    /// Implementations must:
    /// - Use constant-time operations to prevent timing attacks
    /// - Validate all input parameters thoroughly
    /// - Resist signature malleability attacks
    /// - Follow the cryptographic standards for their signature scheme
    ///
    /// # Examples
    ///
    /// ```rust
    /// use soroban_sdk::{Bytes, Env};
    /// use stellar_accounts::verifiers::Verifier;
    ///
    /// // Example usage (implementation-specific)
    /// let is_valid = MyVerifier::verify(&env, message_hash, public_key, signature);
    /// if is_valid {
    ///     // Signature is valid, proceed with authorization
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// No default implementation is provided because cryptographic
    /// verification is entirely scheme-specific. See the
    /// [`ed25519`] and [`webauthn`] sub-modules for reference
    /// implementations.
    fn verify(e: &Env, hash: Bytes, key_data: Self::KeyData, sig_data: Self::SigData) -> bool;

    /// Returns the canonical byte representation of the given key data.
    ///
    /// This method maps key data to a unique canonical identity, allowing
    /// the Smart Account to detect when two different byte representations
    /// refer to the same underlying cryptographic key. This is used during
    /// signer registration to prevent a single key holder from registering
    /// multiple "distinct" signers that share the same cryptographic
    /// identity, which could bypass N-of-M threshold policies.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `key_data` - The public key data in the format expected by this
    ///   verifier.
    ///
    /// # Panics
    ///
    /// Implementations should panic with appropriate error codes when:
    /// - `key_data` is malformed or has invalid length
    /// - `key_data` does not represent a valid key for this scheme
    ///
    /// # Security Requirements
    ///
    /// Implementations must satisfy:
    /// - `canonicalize_key(a) == canonicalize_key(b)` if and only if `a` and
    ///   `b` represent the same cryptographic identity
    /// - All valid representations of a given key must map to the same
    ///   canonical output
    ///
    /// # Notes
    ///
    /// No default implementation is provided because canonicalization logic
    /// is specific to each key format (e.g., compressed vs. uncompressed
    /// public keys). See the [`ed25519`] and [`webauthn`]
    /// sub-modules for reference implementations.
    fn canonicalize_key(e: &Env, key_data: Self::KeyData) -> Bytes;

    /// Canonicalizes a batch of keys in one call. Returns a vector of canonical
    /// key bytes. The output order must match input.
    ///
    /// This method is a batched variant of [`Verifier::canonicalize_key`]. It
    /// allows callers to canonicalize multiple keys for the same verifier in a
    /// single cross-contract invocation, reducing overhead and making duplicate
    /// detection more efficient.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `key_data` - A vector of key values to canonicalize.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because it delegates to the
    /// scheme-specific [`Verifier::canonicalize_key`]. There is no
    /// corresponding storage function — implementations typically iterate
    /// over `key_data` and call [`Verifier::canonicalize_key`] for each
    /// entry.
    fn batch_canonicalize_key(e: &Env, key_data: Vec<Self::KeyData>) -> Vec<Bytes>;
}

// A `VerifierClientInterface` must be declared here instead of using the
// public trait above, because traits with associated types are not supported
// by the `#[contractclient]` macro. While this may appear redundant, it is a
// necessary workaround: an identical internal trait is declared with the macro
// to generate the required client implementation. Interaction should occur
// through the public `Verifier` trait above.
#[allow(unused)]
#[contractclient(name = "VerifierClient")]
trait VerifierClientInterface {
    fn verify(e: &Env, hash: Bytes, key_data: Val, sig_data: Val) -> bool;
    fn canonicalize_key(e: &Env, key_data: Val) -> Bytes;
    fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes>;
}
