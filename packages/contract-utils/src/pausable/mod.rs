//! Pausable Contract Module.
//!
//! This contract module allows implementing a configurable stop mechanism for
//! your contract.
//!
//! By implementing the trait [`Pausable`] for your contract, you can safely
//! integrate the Pausable functionality. The trait [`Pausable`] has the
//! following methods:
//! - [`paused()`]
//! - [`pause()`]
//! - [`unpause()`]
//!
//! The trait ensures all the required methods are implemented for your
//! contract, and nothing is forgotten. Additionally, if you are to implement
//! multiple extensions or utilities for your contract, the code will be better
//! organized.
//!
//! We also provide two macros `when_paused` and `when_not_paused`. These macros
//! act as guards for your functions. For example:
//!
//! ```ignore
//! #[when_not_paused]
//! fn transfer(e: &env, from: Address, to: Address) {
//!     /* this body will execute ONLY when NOT_PAUSED */
//! }
//! ```
//!
//! For a safe pause/unpause implementation, we expose the underlying functions
//! required for the pausing. These functions work with the Soroban environment
//! required for the Smart Contracts `e: &Env`, and take advantage of the
//! storage by storing a flag for the pause mechanism.
//!
//! We expect you to utilize these functions (`storage::*`) for implementing the
//! methods of the `Pausable` trait, along with your custom business logic
//! (authentication, etc.)
//!
//! You can opt-out of [`Pausable`] trait, and use `storage::*` functions
//! directly in your contract if you want more customizability. But we encourage
//! the use of [`Pausable`] trait instead, due to the following reasons:
//! - there is no additional cost
//! - standardization
//! - you cannot mistakenly forget one of the methods
//! - your code will be better organized, especially if you implement multiple
//!   extensions/utils
//!
//! TL;DR
//! to see it all in action, check out the `examples/pausable/src/contract.rs`
//! file.

mod storage;

mod test;

use soroban_sdk::{contracterror, symbol_short, Address, Env};

pub use crate::pausable::storage::{pause, paused, unpause, when_not_paused, when_paused};

pub trait Pausable {
    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn paused(e: &Env) -> bool {
        crate::pausable::paused(e)
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
    /// We recommend using [`pause`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`pause`]
    /// intentionally lacks authorization controls. If you want to restrict
    /// who can `pause` the contract, you MUST implement proper
    /// authorization in your contract.
    fn pause(e: &Env, caller: &soroban_sdk::Address);

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
    /// We recommend using [`unpause`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The base implementation of [`unpause`]
    /// intentionally lacks authorization controls. If you want to restrict
    /// who can `unpause` the contract, you MUST implement proper
    /// authorization in your contract.
    fn unpause(e: &Env, caller: &soroban_sdk::Address);

    /// Helper to make a function callable only when the contract is NOT paused.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`PausableError::EnforcedPause`] - Occurs when the contract is already in
    ///   `Paused` state.
    #[internal]
    fn when_not_paused(e: &Env) {
        if Self::paused(e) {
            panic_with_error!(e, PausableError::EnforcedPause);
        }
    }

    /// Helper to make a function callable only when the contract is paused.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`PausableError::ExpectedPause`] - Occurs when the contract is already in
    ///   `Unpaused` state.
    #[internal]
    fn when_paused(e: &Env) {
        if !Self::paused(e) {
            panic_with_error!(e, PausableError::ExpectedPause);
        }
    }
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

/// Emits an event when `Paused` state is triggered.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Events
///
/// * topics - `["paused"]`
/// * data - `[]`
pub fn emit_paused(e: &Env) {
    let topics = (symbol_short!("paused"),);
    e.events().publish(topics, ())
}

/// Emits an event when `Unpaused` state is triggered.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Events
///
/// * topics - `["unpaused"]`
/// * data - `[]`
pub fn emit_unpaused(e: &Env) {
    let topics = (symbol_short!("unpaused"),);
    e.events().publish(topics, ())
}
