use soroban_sdk::{contracttype, panic_with_error, BytesN, Env, Vec};
use stellar_constants::{MERKLE_CLAIMED_EXTEND_AMOUNT, MERKLE_CLAIMED_TTL_THRESHOLD};
use stellar_crypto::{hasher::Hasher, merkle::Verifier};

use crate::{
    merkle_distributor::{emit_set_claimed, emit_set_root, MerkleDistributorError},
    MerkleDistributor,
};

/// Storage keys for the data associated with `MerkleDistributor`
#[contracttype]
pub enum MerkleDistributorStorageKey {
    /// The Merkle root of the distribution tree
    Root,
    /// Maps a leaf hash to its claimed status
    Claimed(BytesN<32>),
}

impl<H> MerkleDistributor<H>
where
    H: Hasher<Output = BytesN<32>>,
{
    /// Returns the Merkle root stored in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::RootNotSet`] - When attempting to get the root
    ///   before it has been set.
    pub fn get_root(e: &Env) -> H::Output {
        e.storage()
            .instance()
            .get(&MerkleDistributorStorageKey::Root)
            .unwrap_or_else(|| panic_with_error!(e, MerkleDistributorError::RootNotSet))
    }

    /// Checks if a leaf has been claimed and extends its TTL if it has.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf hash to check.
    pub fn is_claimed(e: &Env, leaf: H::Output) -> bool {
        let key = MerkleDistributorStorageKey::Claimed(leaf);
        if let Some(claimed) = e.storage().persistent().get(&key) {
            e.storage().persistent().extend_ttl(
                &key,
                MERKLE_CLAIMED_TTL_THRESHOLD,
                MERKLE_CLAIMED_EXTEND_AMOUNT,
            );
            claimed
        } else {
            false
        }
    }

    /// Sets the Merkle root for the distribution. Can only be set once.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `root` - The Merkle root to set.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::RootAlreadySet`] - When attempting to set the
    ///   root after it has already been set.
    ///
    /// # Events
    ///
    /// * topics - `["set_root"]`
    /// * data - `[root: Bytes]`
    pub fn set_root(e: &Env, root: H::Output) {
        let key = MerkleDistributorStorageKey::Root;
        if e.storage().instance().has(&key) {
            panic_with_error!(&e, MerkleDistributorError::RootAlreadySet);
        } else {
            e.storage().instance().set(&key, &root);
            emit_set_root(e, root.into());
        }
    }

    /// Verifies a Merkle proof for a leaf and marks it as claimed if valid.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf hash to verify and claim.
    /// * `proof` - The Merkle proof for the leaf.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::LeafAlreadyClaimed`] - When attempting to claim
    ///   a leaf that has already been claimed.
    /// * [`MerkleDistributorError::InvalidProof`] - When the provided Merkle proof
    ///   is invalid.
    /// * refer to [`Self::get_root`] errors.
    pub fn verify_and_set_claimed(e: &Env, leaf: H::Output, proof: Vec<H::Output>) {
        if Self::is_claimed(e, leaf.clone()) {
            panic_with_error!(e, MerkleDistributorError::LeafAlreadyClaimed);
        }

        let root = Self::get_root(e);

        match Verifier::<H>::verify(e, proof, root, leaf.clone()) {
            true => Self::set_claimed(e, leaf),
            false => panic_with_error!(e, MerkleDistributorError::InvalidProof),
        };
    }

    /// Marks a leaf as claimed in storage.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf hash to mark as claimed.
    ///
    /// # Events
    ///
    /// * topics - `["set_claimed"]`
    /// * data - `[leaf: Bytes]`
    pub fn set_claimed(e: &Env, leaf: H::Output) {
        let key = MerkleDistributorStorageKey::Claimed(leaf.clone());
        e.storage().persistent().set(&key, &true);
        emit_set_claimed(e, leaf.into());
    }
}
