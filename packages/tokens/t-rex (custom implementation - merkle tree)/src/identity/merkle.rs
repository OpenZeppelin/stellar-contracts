use crate::identity::IdentityVerifier;
use soroban_sdk::{Address, Env, Vec};

/// Error types for MerkleIdentityVerifier operations
pub enum Error {
    Unauthorized,
    InvalidProof,
    InvalidRoot,
}

/// Merkle proof-based implementation of IdentityVerifier
/// This trait extends the base IdentityVerifier with Merkle proof functionality
/// for scalable claim verification
pub trait MerkleIdentityVerifier: IdentityVerifier {
    /// Updates the Merkle root for claims
    /// Only callable by authorized issuers
    fn update_claims_root(e: &Env, operator: Address, new_root: [u8; 32]);

    /// Updates the Merkle root for revoked claims
    /// Only callable by authorized issuers
    fn update_revoked_root(e: &Env, operator: Address, new_root: [u8; 32]);

    /// Adds a claim topic to the required topics
    fn add_claim_topic(e: &Env, operator: Address, claim_topic: u32);

    /// Removes a claim topic from the required topics
    fn remove_claim_topic(e: &Env, operator: Address, claim_topic: u32);

    /// Gets the list of required claim topics
    fn get_claim_topics(e: &Env) -> Vec<u32>;

    /// Verifies that a claim is valid using a Merkle proof
    /// This function checks if the claim exists in the claims tree
    fn verify_claim_proof(
        e: &Env,
        user_address: Address,
        claim_topic: u32,
        claim_hash: [u8; 32],
        proof: Vec<[u8; 32]>,
    ) -> bool;

    /// Verifies that a claim has not been revoked using a Merkle proof
    /// This function checks if the claim is NOT in the revoked tree
    fn verify_not_revoked(e: &Env, claim_hash: [u8; 32], proof: Vec<[u8; 32]>) -> bool;

    /// Gets the current Merkle root for claims
    fn get_claims_root(e: &Env) -> [u8; 32];

    /// Gets the current Merkle root for revoked claims
    fn get_revoked_root(e: &Env) -> [u8; 32];

    /// Utility to hash a claim for inclusion in the Merkle tree
    fn hash_claim(
        e: &Env,
        subject: Address,
        topic: u32,
        data: Vec<u8>,
        signature: Vec<u8>,
    ) -> [u8; 32];
}
