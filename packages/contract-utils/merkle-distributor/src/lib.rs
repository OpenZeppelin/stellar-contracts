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
//! - Each node (leaf) **MUST** include an indexable field of type `u32` and
//!   implement the `IndexableLeaf`.
//! - Aside from the `index`, the node can contain any additional fields, with
//!   any names and types, depending on the specific use case (e.g., `address`,
//!   `amount`, `token_id`, etc.).
//! - When constructing the Merkle tree, ensure that the `index` values are
//!   unique and consecutive (or at least unique).
//!
//! ### Example
//!
//! ```ignore,rust
//! use soroban_sdk::contracttype;
//! use stellar_merkle_distributor::IndexableLeaf;
//!
//! #[contracttype]
//! struct LeafData {
//!     pub index: u32,
//!     pub address: Address,
//!     pub amount: i128,
//! }
//!
//! impl IndexableLeaf for LeafData {
//!     fn index(&self) -> u32 {
//!         self.index
//!     }
//! }
//! ```
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
        emit_set_claimed, emit_set_root, IndexableLeaf, MerkleDistributor, MerkleDistributorError,
    },
    storage::MerkleDistributorStorageKey,
};
