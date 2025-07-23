//! This module deals with verification of Merkle Tree proofs. It was adapted
//! from [rust-contracts-stylus](https://github.com/OpenZeppelin/rust-contracts-stylus/blob/main/lib/crypto/src/merkle.rs) to work with Soroban contract.
use core::marker::PhantomData;

use soroban_sdk::{panic_with_error, BytesN, Env, Vec};

use crate::crypto::{
    error::CryptoError,
    hashable::{commutative_hash_pair, hash_pair},
    hasher::Hasher,
};

/// Alias type for `BytesN<32>`
pub type Bytes32 = BytesN<32>;

/// Verify merkle proofs.
pub struct Verifier<H: Hasher>(PhantomData<H>);

impl<H> Verifier<H>
where
    H: Hasher<Output = Bytes32>,
{
    /// Verify that `leaf` is part of a Merkle tree defined by `root` by using
    /// `proof` and a custom hashing algorithm defined by `Hasher`.
    ///
    /// A new root is rebuilt by traversing up the Merkle tree. The `proof`
    /// provided must contain sibling hashes on the branch starting from the
    /// leaf to the root of the tree. Each pair of leaves and each pair of
    /// pre-images are assumed to be sorted.
    ///
    /// A `proof` is valid if and only if the rebuilt hash matches the root
    /// of the tree.
    ///
    /// The tree and the proofs by using keccak256 can be generated with
    /// `OpenZeppelin`'s [merkle tree library](https://github.com/OpenZeppelin/merkle-tree).
    /// WARNING: You should avoid using leaf values that are 64 bytes long
    /// prior to hashing. This is because the concatenation of a sorted pair
    /// of internal nodes in the Merkle tree could be reinterpreted as a
    /// leaf value. `OpenZeppelin`'s JavaScript library generates Merkle trees
    /// that are safe against this attack out of the box.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proof` - A slice of hashes that constitute the merkle proof.
    /// * `root` - The root of the merkle tree, in bytes.
    /// * `leaf` - The leaf of the merkle tree to proof, in bytes.
    #[must_use]
    pub fn verify(e: &Env, proof: Vec<Bytes32>, root: Bytes32, mut leaf: Bytes32) -> bool {
        for hash in proof {
            leaf = commutative_hash_pair(&leaf, &hash, H::new(e));
        }

        leaf == root
    }

    /// Verify that `leaf` is part of a Merkle tree defined by `root` by using
    /// `proof` and a custom hashing algorithm defined by `Hasher`.
    ///
    /// A new root is rebuilt by traversing up the Merkle tree. The `proof`
    /// provided must contain sibling hashes on the branch starting from the
    /// leaf to the root of the tree. There is no assumption about leaves or
    /// nodes being sorted, which differentiates this function from
    /// [`Verifier::verify`]).
    ///
    /// A `proof` is valid if and only if the rebuilt hash matches the root
    /// of the tree.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `proof` - A slice of hashes that constitute the merkle proof.
    /// * `root` - The root of the merkle tree, in bytes.
    /// * `leaf` - The leaf of the merkle tree to proof, in bytes.
    /// * `index` - The 0-based index of the `leaf`.
    ///
    /// # Errors
    ///
    /// * [`CryptoError::MerkleProofOutOfBounds`] - When the length of the proof
    ///   is >= 32.
    /// * [`CryptoError::MerkleIndexOutOfBounds`] - When the index of the leaf
    ///   is out of bounds given the length of the proof.
    #[must_use]
    pub fn verify_with_index(
        e: &Env,
        proof: Vec<Bytes32>,
        root: Bytes32,
        mut leaf: Bytes32,
        mut index: u32,
    ) -> bool {
        // validate proof length and index range
        let len = proof.len();
        if len >= 32 {
            panic_with_error!(e, CryptoError::MerkleProofOutOfBounds)
        }
        if index >= (1 << len) {
            panic_with_error!(e, CryptoError::MerkleIndexOutOfBounds)
        }

        // hash without sorting
        for hash in proof {
            leaf = if index.is_multiple_of(2) {
                hash_pair(&leaf, &hash, H::new(e))
            } else {
                hash_pair(&hash, &leaf, H::new(e))
            };
            index /= 2;
        }

        leaf == root
    }
}
