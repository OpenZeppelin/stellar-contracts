//! Pausable Contract Module.
//!
//! This contract module allows a configurable stop mechanism to be implemented
//! for a contract.
//!
//! By implementing the trait [`Pausable`] for a contract, the Pausable
//! functionality can be integrated safely. The trait [`Pausable`] has the
//! following methods:
//! - [`paused()`]
//! - [`pause()`]
//! - [`unpause()`]
//!
//! The trait ensures that all required methods are implemented for the
//! contract. Additionally, when multiple extensions or utilities are
//! implemented, the code remains better organized.
//!
//! Two macros, `when_paused` and `when_not_paused`, are also provided. These
//! macros act as guards for functions. For example:
//!
//! ```ignore
//! #[when_not_paused]
//! fn transfer(e: &env, from: Address, to: MuxedAddress) {
//!     /* this body will execute ONLY when NOT_PAUSED */
//! }
//! ```
//!
//! For a safe pause/unpause implementation, the underlying functions required
//! for pausing are exposed. These functions work with the Soroban environment
//! required for the Smart Contracts `e: &Env`, and take advantage of the
//! storage by storing a flag for the pause mechanism.
//!
//! These functions (`storage::*`) are intended to be used when implementing
//! the methods of the `Pausable` trait, together with custom business logic
//! (authentication, etc.)
//!
//! The [`Pausable`] trait can be omitted and `storage::*` functions can be
//! used directly in the contract if more customizability is required. The use
//! of [`Pausable`] is still encouraged for the following reasons:
//! - there is no additional cost
//! - standardization
//! - one of the methods cannot be forgotten accidentally
//! - the code remains better organized, especially when multiple
//!   extensions/utils
//!
//! TL;DR
//! to see it all in action, check out the `examples/pausable/src/contract.rs`
//! file.

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, Env};

pub use crate::pausable::storage::{pause, paused, unpause, when_not_paused, when_paused};

#[contracttrait]
pub trait Pausable {
    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn paused(e: &Env) -> bool {
        storage::paused(e)
    }

    /// Triggers `Paused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    ///
    /// # Errors
    ///
    /// * [`PausableError::EnforcedPause`] - Occurs when the contract is already
    ///   in `Paused` state.
    ///
    /// # Events
    ///
    /// * topics - `["paused"]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// [`pause`] is recommended when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`pause`]
    /// intentionally lacks authorization controls. If `pause` access is to be
    /// restricted, proper authorization MUST be implemented in the contract.
    fn pause(e: &Env, caller: Address);

    /// Triggers `Unpaused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    ///
    /// # Errors
    ///
    /// * [`PausableError::ExpectedPause`] - Occurs when the contract is already
    ///   in `Unpaused` state.
    ///
    /// # Events
    ///
    /// * topics - `["unpaused"]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// [`unpause`] is recommended when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`unpause`]
    /// intentionally lacks authorization controls. If `unpause` access is to
    /// be restricted, proper authorization MUST be implemented in the
    /// contract.
    fn unpause(e: &Env, caller: Address);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PausableError {
    /// The operation failed because the contract is paused.
    EnforcedPause = 1000,
    /// The operation failed because the contract is not paused.
    ExpectedPause = 1001,
}

// ################## EVENTS ##################

/// Event emitted when the contract is paused.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Paused {}

/// Emits an event when `Paused` state is triggered.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn emit_paused(e: &Env) {
    Paused {}.publish(e);
}

/// Event emitted when the contract is unpaused.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unpaused {}

/// Emits an event when `Unpaused` state is triggered.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
pub fn emit_unpaused(e: &Env) {
    Unpaused {}.publish(e);
}
