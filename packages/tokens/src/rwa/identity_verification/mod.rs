//! # Identity Verification Module
//!
//! This module groups the contracts involved in investor identity
//! verification:
//!
//! - `claim_issuer`: trusted attestors that sign and validate claims
//! - `claim_topics_and_issuers`: registry of required claim topics and the
//!   issuers allowed to attest them
//! - `identity_claims`: on-chain identity contract that stores claims
//! - `identity_registry_storage`: storage contract that links wallets to
//!   identities and jurisdiction data
//!
//! It also provides the `IdentityVerifier` trait for the contract that ties
//! the full stack together for token checks.
//!
//! ## Architecture & Implementation Approaches
//!
//! Identity verification systems can be implemented in various ways depending
//! on regulatory and business requirements:
//!
//! - **Merkle Tree**: Efficient verification using merkle proofs (minimal
//!   storage)
//! - **Zero-Knowledge**: Privacy-preserving verification (custom ZK circuits)
//! - **Claim-based**: Cryptographic claims from trusted issuers (the default
//!   approach)
//! - and other custom approaches
//!
//! ## Default Implementation
//!
//! The suggested claim-based implementation uses two external contracts:
//! 1. **Claim Topics and Issuers**: Manages trusted issuers and claim types
//! 2. **Identity Registry Storage**: Maps wallet addresses to onchain
//!    identities
//!
//! Since `IdentityRegistryStorage` may not be required for all approaches
//! (e.g., merkle tree or zero-knowledge implementations), it's not part of the
//! trait interface. However, [`storage`] provides the necessary functions for
//! `IdentityRegistryStorage` integration. Examples are available in the RWA
//! examples folder.

use soroban_sdk::{contracttrait, Address, Env};

pub mod claim_issuer;
pub mod claim_topics_and_issuers;
pub mod identity_claims;
pub mod identity_registry_storage;
pub mod storage;

#[cfg(test)]
mod test;

#[contracttrait]
pub trait IdentityVerifier {
    /// Verifies that the identity of an user address has the required valid
    /// claims.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The account to verify.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::RWAError::IdentityVerificationFailed`] - When the
    ///   identity of the account cannot be verified.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because identity verification
    /// is architecture-dependent (claim-based, merkle tree, zero-knowledge,
    /// etc.). For the default claim-based approach, use
    /// [`storage::verify_identity`] for the underlying logic.
    fn verify_identity(e: &Env, account: &Address);

    /// Returns the target address for the recovery process for the old account.
    /// If the old account is not a target of a recovery process, `None` is
    /// returned.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `old_account` - The address of the old account.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because identity verification
    /// is architecture-dependent. For the default claim-based approach, use
    /// [`storage::recovery_target`] for the underlying logic.
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
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling
    /// [`storage::set_claim_topics_and_issuers`] for the implementation.
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address);

    /// Returns the Claim Topics and Issuers contract linked to the token.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::RWAError::ClaimTopicsAndIssuersNotSet`] - When the claim
    ///   topics and issuers contract is not set.
    fn claim_topics_and_issuers(e: &Env) -> Address {
        storage::claim_topics_and_issuers(e)
    }
}
