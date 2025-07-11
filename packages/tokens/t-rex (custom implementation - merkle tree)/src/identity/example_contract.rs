/* Example Implementation */

use crate::identity::identity_verifier::IdentityVerifier;
use crate::identity::merkle::MerkleIdentityVerifier;
use soroban_sdk::{Address, Env};

pub struct MerkleContract;

impl IdentityVerifier for MerkleContract {
    fn is_verified(e: &Env, account: Address) -> bool {
        let claim_topics = self::MerkleIdentityVerifier::get_claim_topics(e);
        for claim_topic in claim_topics {
            let data = fill_data();
            let signature = get_signature_from_offchain(account, claim_topic);

            let claim_hash =
                self::MerkleIdentityVerifier::hash_claim(e, account, claim_topic, data, signature);

            let proof = get_proof_from_offchain(account, claim_topic);

            if !self::MerkleIdentityVerifier::verify_claim_proof(
                e,
                account,
                claim_topic,
                claim_hash,
                proof,
            ) {
                return false;
            }

            if !self::MerkleIdentityVerifier::verify_not_revoked(e, claim_hash, proof) {
                return false;
            }
        }
        true
    }
}
