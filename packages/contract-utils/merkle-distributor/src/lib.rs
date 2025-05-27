#![no_std]
//! # Merkle Distributor
//!
//! This module implements a Merkle-based claim distribution system using Merkle
//! proofs for verification.
//!
//! ## Implementation Notes
//!
//! Claims are **indexed by a `u32` index**, corresponding to the position of
//! each leaf in the original Merkle tree.
//!
//! ### Requirements for Leaf Structure
//!
//! - Each node (leaf) **MUST** include an `index` field of type `u32`.
//! - Aside from the `index`, it can contain any additional fields, with any
//!   names and types, depending on the specific use case (e.g., `address`,
//!   `amount`, `token_id`, etc.).
//! - When constructing the Merkle tree, ensure that the `index` values are
//!   unique and consecutive (or at least unique).
//!
//! This structure supports a wide variety of distribution mechanisms such as:
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
