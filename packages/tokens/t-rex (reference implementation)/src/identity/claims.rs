use soroban_sdk::{Address, Env, Vec};

/// Interface for managing claims for identities
/// This trait provides functionality for managing claims
/// and is designed to be bound to a ClaimTopicsAndIssuers contract
pub trait IdentityClaims {
    /// Gets a specific claim for an identity
    /// Anyone can call this function
    /// Does not return revoked claims
    fn get_claims(e: &Env, identity: Address) -> Result<Vec<Vec<u8>>, Error>;

    /// Gets all claims for an identity by topic
    /// Anyone can call this function
    /// Does not return revoked claims
    fn get_claims_by_topic(e: &Env, identity: Address, claim_topic: u32) -> Vec<Vec<u8>>;

    /// Adds a claim for an identity
    /// Only callable by authorized issuers
    /// Includes verification logic and cross-contract calls to validate topics and issuers
    fn add_claim(
        e: &Env,
        issuer: Address,
        identity: Address,
        claim_topic: u32,
        data: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), Error>;

    /// Removes a claim from an identity
    /// Only callable by authorized issuers
    /// Includes verification logic and cross-contract calls
    fn remove_claim(
        e: &Env,
        issuer: Address,
        identity: Address,
        claim_topic: u32,
        index: u32,
    ) -> Result<(), Error>;

    /// Revokes a claim for an identity
    /// Only callable by authorized issuers
    /// Includes verification logic and cross-contract calls
    fn revoke_claim(
        e: &Env,
        issuer: Address,
        identity: Address,
        claim_topic: u32,
        index: u32,
    ) -> Result<(), Error>;

    /// Gets the bound ClaimTopicsAndIssuers contract
    fn get_claim_topics_and_issuers(e: &Env) -> Address;
}
