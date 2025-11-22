//! # Ownable Contract Module.
//!
//! This module introduces a simple access control mechanism where a contract
//! has an account (owner) that can be granted exclusive access to specific
//! functions.
//!
//! The `Ownable` trait exposes methods for:
//! - Getting the current owner
//! - Transferring ownership
//! - Renouncing ownership
//!
//! The helper `enforce_owner_auth()` is available to restrict access to only
//! the owner. You can also use the `#[only_owner]` macro (provided elsewhere)
//! to simplify this.
//!
//! ```ignore
//! #[only_owner]
//! fn set_config(e: &Env, new_config: u32) { ... }
//! ```
//!
//! See `examples/ownable/src/contract.rs` for a working example.
//!
//! ## Note
//!
//! The ownership transfer is processed in 2 steps:
//!
//! 1. Initiating the ownership transfer by the current owner
//! 2. Accepting the ownership by the designated owner
//!
//! Not providing a direct ownership transfer is a deliberate design decision to
//! help avoid mistakes by transferring to a wrong address.

mod storage;

#[cfg(test)]
mod test;

#[cfg(feature = "certora")]
pub mod specs;

use soroban_sdk::{contracterror, Address, Env};

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::{contractevent};


pub use crate::ownable::storage::{
    accept_ownership, enforce_owner_auth, get_owner, renounce_ownership, set_owner,
    transfer_ownership, OwnableStorageKey,
};

/// A trait for managing contract ownership using a 2-step transfer pattern.
///
/// Provides functions to query ownership, initiate a transfer, or renounce
/// ownership.
pub trait Ownable {
    /// Returns `Some(Address)` if ownership is set, or `None` if ownership has
    /// been renounced.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_owner(e: &Env) -> Option<Address>;

    /// Initiates a 2-step ownership transfer to a new address.
    ///
    /// Requires authorization from the current owner. The new owner must later
    /// call `accept_ownership()` to complete the transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `new_owner` - The proposed new owner.
    /// * `live_until_ledger` - Ledger number until which the new owner can
    ///   accept. A value of `0` cancels any pending transfer.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::OwnerNotSet`] - If the owner is not set.
    /// * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
    ///   trying to cancel a transfer that doesn't exist.
    /// * [`crate::role_transfer::RoleTransferError::InvalidLiveUntilLedger`] -
    ///   If the specified ledger is in the past.
    /// * [`crate::role_transfer::RoleTransferError::InvalidPendingAccount`] -
    ///   If the specified pending account is not the same as the provided `new`
    ///   address.
    ///
    /// # Notes
    ///
    /// * Authorization for the current owner is required.
    fn transfer_ownership(e: &Env, new_owner: Address, live_until_ledger: u32);

    /// Accepts a pending ownership transfer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::role_transfer::RoleTransferError::NoPendingTransfer`] - If
    ///   there is no pending transfer to accept.
    ///
    /// # Events
    ///
    /// * topics - `["ownership_transfer_completed"]`
    /// * data - `[new_owner: Address]`
    fn accept_ownership(e: &Env);

    /// Renounces ownership of the contract.
    ///
    /// Permanently removes the owner, disabling all functions gated by
    /// `#[only_owner]`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::TransferInProgress`] - If there is a pending ownership
    ///   transfer.
    /// * [`OwnableError::OwnerNotSet`] - If the owner is not set.
    ///
    /// # Notes
    ///
    /// * Authorization for the current owner is required.
    fn renounce_ownership(e: &Env);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OwnableError {
    OwnerNotSet = 2100,
    TransferInProgress = 2101,
    OwnerAlreadySet = 2102,
}

// ################## EVENTS ##################

/// Event emitted when an ownership transfer is initiated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipTransfer {
    pub old_owner: Address,
    pub new_owner: Address,
    pub live_until_ledger: u32,
}

/// Emits an event when an ownership transfer is initiated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The address of the current owner.
/// * `new_owner` - The address of the proposed new owner.
/// * `live_until_ledger` - The ledger number until which the new owner can
///   accept the transfer.
#[cfg(not(feature = "certora"))]
pub fn emit_ownership_transfer(
    e: &Env,
    old_owner: &Address,
    new_owner: &Address,
    live_until_ledger: u32,
) {
    OwnershipTransfer {
        old_owner: old_owner.clone(),
        new_owner: new_owner.clone(),
        live_until_ledger,
    }
    .publish(e);
}

/// Event emitted when an ownership transfer is completed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipTransferCompleted {
    pub new_owner: Address,
}

/// Emits an event when an ownership transfer is completed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `new_owner` - The address of the new owner.
#[cfg(not(feature = "certora"))]
pub fn emit_ownership_transfer_completed(e: &Env, new_owner: &Address) {
    OwnershipTransferCompleted { new_owner: new_owner.clone() }.publish(e);
}

/// Event emitted when ownership is renounced.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipRenounced {
    pub old_owner: Address,
}

/// Emits an event when ownership is renounced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The address of the owner who renounced ownership.
#[cfg(not(feature = "certora"))]
pub fn emit_ownership_renounced(e: &Env, old_owner: &Address) {
    OwnershipRenounced { old_owner: old_owner.clone() }.publish(e);
}
