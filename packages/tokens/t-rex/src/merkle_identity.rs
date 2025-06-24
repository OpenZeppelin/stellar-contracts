use soroban_sdk::{Address, Env};
use ClaimTopics::ClaimTopics;
use Identity::IdentityVerifier;

// Merkle proof-based implementation of IdentityVerifier
pub trait MerkleIdentityVerifier: IdentityVerifier + ClaimTopics {
    // Updates the Merkle root for claims
    fn update_claims_root(e: &Env, operator: Address, new_root: [u8; 32]);

    // Updates the Merkle root for revoked claims
    fn update_revoked_root(e: &Env, operator: Address, new_root: [u8; 32]);

    // Verifies that a claim is valid using a Merkle proof
    fn verify_claim_proof(
        e: &Env,
        user_address: Address,
        claim_topic: u32,
        claim_hash: [u8; 32],
        proof: Vec<[u8; 32]>,
    ) -> bool;

    // Verifies that a claim has not been revoked using a Merkle proof
    fn verify_not_revoked(e: &Env, claim_hash: [u8; 32], proof: Vec<[u8; 32]>) -> bool;

    // Gets the current Merkle roots
    fn get_claims_root(e: &Env) -> [u8; 32];
    fn get_revoked_root(e: &Env) -> [u8; 32];

    // Utility to hash a claim for inclusion in the Merkle tree
    fn hash_claim(
        e: &Env,
        subject: Address,
        topic: u32,
        data: Vec<u8>,
        signature: Vec<u8>,
    ) -> [u8; 32];
}
