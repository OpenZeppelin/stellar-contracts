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

mod test;

use soroban_sdk::{contracterror, Address, Env, Symbol};

pub use crate::ownable::storage::{
    accept_ownership, enforce_owner_auth, get_owner, renounce_ownership, set_owner,
    transfer_ownership, OwnableStorageKey,
};

/// A trait for managing contract ownership using a 2-step transfer pattern.
///
/// Provides functions to query ownership, initiate a transfer, or renounce
/// ownership.
#[contracttrait(default = Owner, is_extension = true)]
pub trait Ownable {
    /// Returns `Some(Address)` if ownership is set, or `None` if ownership has
    /// been renounced.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_owner(e: &Env) -> Option<soroban_sdk::Address>;

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
    fn transfer_ownership(e: &Env, new_owner: &soroban_sdk::Address, live_until_ledger: u32);

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

    /// Enforces authorization from the current owner.
    ///
    /// This is used internally by the `#[only_owner]` macro expansion to gate
    /// access.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::OwnerNotSet`] - If the owner is not set.
    #[internal]
    fn only_owner(e: &soroban_sdk::Env) {
        let Some(owner) = Self::get_owner(e) else {
            panic_with_error!(e, OwnableError::OwnerNotSet);
        };
        owner.require_auth()
    }

    #[internal]
    fn enforce_owner_auth(e: &soroban_sdk::Env) {
        enforce_owner_auth(e);
    }

    /// Sets owner role.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The account to grant the owner privilege.
    ///
    /// # Errors
    ///
    /// * [`OwnableError::OwnerAlreadySet`] - If the owner is already set.
    ///
    /// **IMPORTANT**: this function lacks authorization checks.
    /// It is expected to call this function only in the constructor!
    #[internal]
    fn set_owner(e: &soroban_sdk::Env, owner: &soroban_sdk::Address);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OwnableError {
    OwnerNotSet = 1220,
    TransferInProgress = 1221,
    OwnerAlreadySet = 1222,
}

// ################## EVENTS ##################

/// Emits an event when an ownership transfer is initiated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The current owner initiating the transfer.
/// * `new_owner` - The proposed new owner.
/// * `live_until_ledger` - The ledger number at which the pending transfer will
///   expire. If this value is `0`, it means the pending transfer is cancelled.
///
/// # Events
///
/// * topics - `["ownership_transfer"]`
/// * data - `[old_owner: Address, new_owner: Address]`
pub fn emit_ownership_transfer(
    e: &Env,
    old_owner: &Address,
    new_owner: &Address,
    live_until_ledger: u32,
) {
    let topics = (Symbol::new(e, "ownership_transfer"),);
    e.events().publish(topics, (old_owner, new_owner, live_until_ledger));
}

/// Emits an event when an ownership transfer is completed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `new_owner` - The new owner who accepted the transfer.
///
/// # Events
///
/// * topics - `["ownership_transfer_completed"]`
/// * data - `[new_owner: Address]`
pub fn emit_ownership_transfer_completed(e: &Env, new_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_transfer_completed"),);
    e.events().publish(topics, new_owner);
}

/// Emits an event when ownership is renounced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `old_owner` - The address of the owner who renounced ownership.
///
/// # Events
///
/// * topics - `["ownership_renounced"]`
/// * data - `[old_owner: Address]`
pub fn emit_ownership_renounced(e: &Env, old_owner: &Address) {
    let topics = (Symbol::new(e, "ownership_renounced"),);
    e.events().publish(topics, old_owner);
}
