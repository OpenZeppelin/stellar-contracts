#![no_std]
//! # Merkle Distributor
//!
//! This module implements a Merkle-based distribution system where claims are
//! stored and verified using Merkle proofs.
//!
//! ## Implementation Notes
//!
//! Each claim is **indexed by the hash of the leaf node** in the Merkle tree.
//! This means:
//!
//! - Every leaf must be unique, as duplicate leaves will result in identical
//!   hashes and would overwrite or conflict with existing claims.
//! - Indexing by leaf hash allows flexibility in the leaf structure, meaning
//!   any custom data structure (e.g., index + address + amount, address +
//!   metadata, etc.) can be used as long as it's hashed consistently.
//!
//! This design makes the distributor highly adaptable for various use cases
//! such as:
//!
//! - Token airdrops
//! - NFT distributions
//! - Off-chain allowlists
//! - Snapshot-based voting
//! - Custom claim logic involving metadata

mod merkle_distributor;
mod storage;
mod test;

pub use crate::{
    merkle_distributor::{
        emit_set_claimed, emit_set_root, MerkleDistributor, MerkleDistributorError,
    },
    storage::MerkleDistributorStorageKey,
};
