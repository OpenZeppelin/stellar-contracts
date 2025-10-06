use soroban_sdk::{contractclient, Address, Env};

#[cfg(test)]
mod test;

pub mod storage;

/// # Identity Verifier Module
///
/// This module provides the `IdentityVerifier` trait for verifying user
/// identities in Real World Asset (RWA) tokens. Identity verification is
/// critical for regulatory compliance, ensuring only verified users can
/// participate in token operations by validating addresses against
/// cryptographic claims from trusted authorities.
///
/// ## Architecture & Implementation Approaches
///
/// Identity verification systems can be implemented in various ways depending
/// on regulatory and business requirements:
///
/// - **Merkle Tree**: Efficient verification using merkle proofs (minimal
///   storage)
/// - **Zero-Knowledge**: Privacy-preserving verification (custom ZK circuits)
/// - **Claim-based**: Cryptographic claims from trusted issuers (our default
///   approach)
/// - and other custom approaches
///
/// ## Default Implementation
///
/// Our suggested claim-based implementation uses two external contracts:
/// 1. **Claim Topics and Issuers**: Manages trusted issuers and claim types
/// 2. **Identity Registry Storage**: Maps wallet addresses to onchain
///    identities
///
/// Since `IdentityRegistryStorage` may not be required for all approaches
/// (e.g., merkle tree or zero-knowledge implementations), it's not part of the
/// trait interface. However, `storage.rs` provides the necessary functions for
/// `IdentityRegistryStorage` integration. Examples are available in the RWA
/// examples folder.
#[contractclient(name = "IdentityVerifierClient")]
pub trait IdentityVerifier {
    /// Verifies that the identity of an user address has the required valid
    /// claims.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The user address to verify.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::RWAError::IdentityVerificationFailed`] - When the
    ///   identity of the user address cannot be verified.
    fn verify_identity(e: &Env, user_address: &Address);

    /// Returns the target address for the recovery process for the old account.
    /// If the old account is not a target of a recovery process, `None` is
    /// returned.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `old_account` - The address of the old account.
    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address>;

    /// Sets the identity registry contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topics_and_issuers` - The address of the claim topics and
    ///   issuers contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["claim_topics_issuers_set", claim_topics_and_issuers:
    ///   Address]`
    /// * data - `[]`
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address);

    /// Returns the Claim Topics and Issuers contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::RWAError::ClaimTopicsAndIssuersNotSet`] - When the claim
    ///   topics and issuers contract is not set.
    fn claim_topics_and_issuers(e: &Env) -> Address;
}
