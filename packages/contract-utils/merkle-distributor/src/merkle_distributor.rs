use core::marker::PhantomData;

use soroban_sdk::{contracterror, symbol_short, Bytes, Env, Symbol};
use stellar_crypto::hasher::Hasher;

pub struct MerkleDistributor<H: Hasher>(PhantomData<H>);

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MerkleDistributorError {
    /// The merkle root is not set.
    RootNotSet = 1200,
    /// The merkle root is aleasy set.
    RootAlreadySet = 1201,
    /// The provided leaf was already claimed.
    LeafAlreadyClaimed = 1202,
    /// The proof is invalid.
    InvalidProof = 1203,
}

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

/// Emits an event when a leaf is claimed.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `leaf` - The leaf to be claimed.
///
/// # Events
///
/// * topics - `["set_claimed"]`
/// * data - `[leaf: Bytes]`
pub fn emit_set_claimed(e: &Env, leaf: Bytes) {
    let topics = (Symbol::new(e, "set_claimed"),);
    e.events().publish(topics, leaf)
}
