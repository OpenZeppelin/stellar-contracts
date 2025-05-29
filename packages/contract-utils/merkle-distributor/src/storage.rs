use soroban_sdk::{contracttype, panic_with_error, xdr::ToXdr, BytesN, Env, Vec};
use stellar_constants::{MERKLE_CLAIMED_EXTEND_AMOUNT, MERKLE_CLAIMED_TTL_THRESHOLD};
use stellar_crypto::{hasher::Hasher, merkle::Verifier};

use crate::{
    merkle_distributor::{emit_set_claimed, emit_set_root, IndexableNode, MerkleDistributorError},
    MerkleDistributor,
};

/// Storage keys for the data associated with `MerkleDistributor`
#[contracttype]
pub enum MerkleDistributorStorageKey {
    /// The Merkle root of the distribution tree
    Root,
    /// Maps an index to its claimed status
    Claimed(u32),
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
    /// * [`MerkleDistributorError::RootNotSet`] - When attempting to get the
    ///   root before it has been set.
    pub fn get_root(e: &Env) -> H::Output {
        e.storage()
            .instance()
            .get(&MerkleDistributorStorageKey::Root)
            .unwrap_or_else(|| panic_with_error!(e, MerkleDistributorError::RootNotSet))
    }

    /// Checks if an index has been claimed and extends its TTL if it has.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `index` - The index to check.
    pub fn is_claimed(e: &Env, index: u32) -> bool {
        let key = MerkleDistributorStorageKey::Claimed(index);
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
    /// * [`MerkleDistributorError::RootAlreadySet`] - When attempting to set
    ///   the root after it has already been set.
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

    /// Marks an index as claimed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `index` - The index to mark as claimed.
    ///
    /// # Events
    ///
    /// * topics - `["set_claimed"]`
    /// * data - `[index: u32]`
    pub fn set_claimed(e: &Env, index: u32) {
        let key = MerkleDistributorStorageKey::Claimed(index);
        e.storage().persistent().set(&key, &true);
        emit_set_claimed(e, index.into());
    }

    /// Verifies a Merkle proof for a node and marks its index as claimed if the
    /// proof is valid.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `node` - The node data containing an index field.
    /// * `proof` - The Merkle proof for the node.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::IndexAlreadyClaimed`] - When attempting to
    ///   claim an index that has already been claimed. claim an index that has
    ///   already been claimed.
    /// * [`MerkleDistributorError::InvalidProof`] - When the provided Merkle
    ///   proof is invalid.
    /// * [`MerkleDistributorError::RootNotSet`] - When the root is not set or
    ///   when the node data does not contain a valid index.
    pub fn verify_and_set_claimed<N: ToXdr + IndexableNode>(
        e: &Env,
        node: N,
        proof: Vec<H::Output>,
    ) {
        let index = node.index();
        let encoded = node.to_xdr(e);

        // Check if already claimed
        if Self::is_claimed(e, index) {
            panic_with_error!(e, MerkleDistributorError::IndexAlreadyClaimed);
        }

        // Verify proof
        let root = Self::get_root(e);
        let mut hasher = H::new(e);
        hasher.update(encoded);
        let leaf = hasher.finalize();

        match Verifier::<H>::verify(e, proof, root, leaf) {
            true => Self::set_claimed(e, index),
            false => panic_with_error!(e, MerkleDistributorError::InvalidProof),
        };
    }
}
