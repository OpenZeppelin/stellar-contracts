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

mod storage;

mod test;

pub use crate::merkle_distributor::storage::MerkleDistributorStorageKey;

use core::marker::PhantomData;
use soroban_sdk::{contracterror, symbol_short, Bytes, Env, Symbol, Val};

use crate::crypto::hasher::Hasher;

pub trait IndexableLeaf {
    fn index(&self) -> u32;
}

pub struct MerkleDistributor<H: Hasher>(PhantomData<H>);

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MerkleDistributorError {
    /// The merkle root is not set.
    RootNotSet = 1300,
    /// The provided index was already claimed.
    IndexAlreadyClaimed = 1301,
    /// The proof is invalid.
    InvalidProof = 1302,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const MERKLE_CLAIMED_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const MERKLE_CLAIMED_TTL_THRESHOLD: u32 = MERKLE_CLAIMED_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Emits an event when a merkle root is set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `root` - The root to be set.
///
/// # Events
///
/// * topics - `["set_root"]`
/// * data - `[root: Bytes]`
pub fn emit_set_root(e: &Env, root: Bytes) {
    let topics = (symbol_short!("set_root"),);
    e.events().publish(topics, root)
}

/// Emits an event when an index is claimed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `index` - The index that was claimed.
///
/// # Events
///
/// * topics - `["set_claimed"]`
/// * data - `[index: u32]`
pub fn emit_set_claimed(e: &Env, index: Val) {
    let topics = (Symbol::new(e, "set_claimed"),);
    e.events().publish(topics, index)
}
