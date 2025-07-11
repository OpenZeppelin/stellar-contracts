use crate::identity::IdentityVerifier;
use soroban_sdk::{Address, Env, Vec};

/// Error types for IdentityRegistry operations
pub enum Error {
    Unauthorized,
    IdentityNotFound,
    IdentityAlreadyExists,
}

/// Identity Registry for managing onchain identities
/// This trait extends the base IdentityVerifier and provides
/// functionality for managing identity contracts
pub trait IdentityRegistry: IdentityVerifier {
    /// Adds an identity for a user
    /// Maps a user address to their onchain identity contract
    /// Only callable by authorized roles
    fn add_identity(
        e: &Env,
        operator: Address,
        user_address: Address,
        identity_contract: Address,
    ) -> Result<(), Error>;

    /// Gets the onchain identity contract for a user
    fn get_identity_onchain_id(e: &Env, user_address: Address) -> Option<Address>;

    /// Removes an identity for a user
    /// Only callable by authorized roles
    fn remove_identity(e: &Env, operator: Address, user_address: Address) -> Result<(), Error>;

    /// Modifies an identity for a user
    /// Only callable by authorized roles
    fn modify_identity(
        e: &Env,
        operator: Address,
        user_address: Address,
        new_identity_contract: Address,
    ) -> Result<(), Error>;

    /// Adds a claim topic to the list of trusted topics
    /// Only callable by authorized roles
    fn add_claim_topic(e: &Env, operator: Address, claim_topic: u32) -> Result<(), Error>;

    /// Removes a claim topic from the list of trusted topics
    /// Only callable by authorized roles
    fn remove_claim_topic(e: &Env, operator: Address, claim_topic: u32) -> Result<(), Error>;

    /// Gets the list of trusted claim topics
    fn get_claim_topics(e: &Env) -> Vec<u32>;

    /// Adds a trusted issuer for a specific claim topic
    /// Only callable by authorized roles
    fn add_trusted_issuer(
        e: &Env,
        operator: Address,
        issuer: Address,
        claim_topics: Vec<u32>,
    ) -> Result<(), Error>;

    /// Removes a trusted issuer
    /// Only callable by authorized roles
    fn remove_trusted_issuer(e: &Env, operator: Address, issuer: Address) -> Result<(), Error>;

    /// Updates the claim topics for a trusted issuer
    /// Only callable by authorized roles
    fn update_issuer_claim_topics(
        e: &Env,
        operator: Address,
        issuer: Address,
        claim_topics: Vec<u32>,
    ) -> Result<(), Error>;

    /// Gets all trusted issuers
    fn get_trusted_issuers(e: &Env) -> Vec<Address>;

    /// Gets trusted issuers for a specific claim topic
    fn get_trusted_issuers_for_claim_topic(e: &Env, claim_topic: u32) -> Vec<Address>;

    /// Checks if an issuer is trusted
    fn is_trusted_issuer(e: &Env, issuer: Address) -> bool;

    /// Gets the claim topics that a trusted issuer is allowed to issue
    fn get_trusted_issuer_claim_topics(e: &Env, issuer: Address) -> Result<Vec<u32>, Error>;

    /// Checks if a trusted issuer is allowed to issue a specific claim topic
    fn has_claim_topic(e: &Env, issuer: Address, claim_topic: u32) -> bool;
}
