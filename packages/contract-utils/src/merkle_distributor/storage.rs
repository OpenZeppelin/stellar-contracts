use soroban_sdk::{contracttype, panic_with_error, xdr::ToXdr, BytesN, Env, Vec};

#[cfg(not(feature = "certora"))]
use crate::{merkle_distributor::{emit_set_claimed, emit_set_root}};

use crate::{
    crypto::{hasher::Hasher, merkle::Verifier},
    merkle_distributor::{
        IndexableLeaf, MerkleDistributor, MerkleDistributorError,
        MERKLE_CLAIMED_EXTEND_AMOUNT, MERKLE_CLAIMED_TTL_THRESHOLD,
    },
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

    /// Sets the Merkle root for the distribution.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `root` - The Merkle root to set.
    ///
    /// # Events
    ///
    /// * topics - `["set_root"]`
    /// * data - `[root: Bytes]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function lacks authorization checks and should
    /// only be used:
    /// - During contract initialization/construction
    /// - In admin functions that implement their own authorization logic
    pub fn set_root(e: &Env, root: H::Output) {
        let key = MerkleDistributorStorageKey::Root;
        e.storage().instance().set(&key, &root);
        #[cfg(not(feature = "certora"))]
        emit_set_root(e, root.into());
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
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function lacks authorization checks and should only
    /// be used in admin functions that implement their own authorization logic.
    pub fn set_claimed(e: &Env, index: u32) {
        let key = MerkleDistributorStorageKey::Claimed(index);
        e.storage().persistent().set(&key, &true);
        #[cfg(not(feature = "certora"))]
        emit_set_claimed(e, index.into());
    }

    /// Verifies a Merkle proof for a leaf and marks its index as claimed if the
    /// proof is valid. Internally using [`Verifier::verify`] which assumes that
    /// when the tree gets constructed, **commutative** hashing was used,
    /// i.e.the leaves are **sorted**.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf data containing an index field.
    /// * `proof` - The Merkle proof for the leaf.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::IndexAlreadyClaimed`] - When attempting to
    ///   claim an index that has already been claimed. claim an index that has
    ///   already been claimed.
    /// * [`MerkleDistributorError::InvalidProof`] - When the provided Merkle
    ///   proof is invalid.
    /// * [`MerkleDistributorError::RootNotSet`] - When the root is not set or
    ///   when the leaf data does not contain a valid index.
    pub fn verify_and_set_claimed<N: ToXdr + IndexableLeaf>(
        e: &Env,
        leaf: N,
        proof: Vec<H::Output>,
    ) {
        let (root, leaf_hash, index) = Self::get_verification_args(e, leaf);

        // Check if already claimed
        if Self::is_claimed(e, index) {
            panic_with_error!(e, MerkleDistributorError::IndexAlreadyClaimed);
        }

        // Verify proof
        match Verifier::<H>::verify(e, proof, root, leaf_hash) {
            true => Self::set_claimed(e, index),
            false => panic_with_error!(e, MerkleDistributorError::InvalidProof),
        };
    }

    /// Verifies a Merkle proof for a leaf and marks its index as claimed if the
    /// proof is valid. Internally using [`Verifier::verify_with_index`] which
    /// assumes that when the tree gets constructed, **non-commutative** hashing
    /// was used, i.e. the leaves and the nodes are **unsorted**.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf data containing an index field.
    /// * `proof` - The Merkle proof for the leaf.
    ///
    /// # Errors
    ///
    /// * [`MerkleDistributorError::IndexAlreadyClaimed`] - When attempting to
    ///   claim an index that has already been claimed. claim an index that has
    ///   already been claimed.
    /// * [`MerkleDistributorError::InvalidProof`] - When the provided Merkle
    ///   proof is invalid.
    /// * [`MerkleDistributorError::RootNotSet`] - When the root is not set or
    ///   when the leaf data does not contain a valid index.
    pub fn verify_with_index_and_set_claimed<N: ToXdr + IndexableLeaf>(
        e: &Env,
        leaf: N,
        proof: Vec<H::Output>,
    ) {
        let (root, leaf_hash, index) = Self::get_verification_args(e, leaf);

        // Check if already claimed
        if Self::is_claimed(e, index) {
            panic_with_error!(e, MerkleDistributorError::IndexAlreadyClaimed);
        }

        // Verify proof
        match Verifier::<H>::verify_with_index(e, proof, root, leaf_hash, index) {
            true => Self::set_claimed(e, index),
            false => panic_with_error!(e, MerkleDistributorError::InvalidProof),
        };
    }

    /// Internal helper function that returns a tuple of the root, the hashed
    /// leaf and the leaf index.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `leaf` - The leaf data containing an index field.
    fn get_verification_args<N: ToXdr + IndexableLeaf>(
        e: &Env,
        leaf: N,
    ) -> (H::Output, H::Output, u32) {
        let index = leaf.index();
        let encoded = leaf.to_xdr(e);

        let root = Self::get_root(e);
        let mut hasher = H::new(e);
        hasher.update(encoded);
        let leaf_hash = hasher.finalize();

        (root, leaf_hash, index)
    }
}
