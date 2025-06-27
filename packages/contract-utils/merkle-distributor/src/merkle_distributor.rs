use core::marker::PhantomData;

use soroban_sdk::{contracterror, symbol_short, Bytes, Env, Symbol, Val};
use stellar_crypto::hasher::Hasher;

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
