use soroban_sdk::{Address, Env, Vec};
use ClaimTopics::ClaimTopics;
use Identity::IdentityVerifier;

// `operator` below are the Claim Issuers, and we can apply the authorization logic for them
// for example with Access Control.
// A simple implementation of IdentityVerifier with direct storage
pub trait SimpleIdentityVerifier: IdentityVerifier + ClaimTopics {
    // Adds a claim for a user
    //
    // For self-attestations,
    // we can declare some claim_topics,
    // and if the `operator` and `user_address` are the same,
    // we can skip the signature check.
    fn add_claim(
        e: &Env,
        operator: Address,
        user_address: Address,
        claim_topic: u32,
        data: Vec<u8>,
    ) -> Result<(), Error>;

    // Removes a claim from a user
    fn remove_claim(
        e: &Env,
        operator: Address,
        user_address: Address,
        claim_topic: u32,
    ) -> Result<(), Error>;

    // Revokes a claim for a user
    fn revoke_claim(
        e: &Env,
        operator: Address,
        user_address: Address,
        claim_topic: u32,
    ) -> Result<(), Error>;

    // Checks if a user has a specific claim
    fn has_claim(e: &Env, user_address: Address, claim_topic: u32) -> bool;
}
