//! Pausable Contract Module.
//!
//! Contract module which allows implementing an emergency stop mechanism
//! that can be triggered by an authorized account.
//!
//! It provides functions [`pausable::when_not_paused`]
//! and [`pausable::when_paused`],
//! which can be added to the functions of your contract.
//!
//! Note that your contract will NOT be pausable by simply including this
//! module only once and where you use [`pausable::when_not_paused`].

use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Env};

#[contractclient(name = "PausableClient")]
pub trait Pausable {
    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn paused(e: Env) -> bool;

    /// Triggers `Paused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    ///
    /// # Errors
    ///
    /// If the contract is in `Paused` state, then the error
    /// [`PausableError::EnforcedPause`] is thrown.
    ///
    /// # Events
    ///
    /// * topics - `["paused"]`
    /// * data - `[caller: Address]`
    fn pause(e: Env, caller: Address);

    /// Triggers `Unpaused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    ///
    /// # Errors
    ///
    /// If the contract is in `Unpaused` state, then the error
    /// [`PausableError::ExpectedPause`] is thrown.
    ///
    /// # Events
    ///
    /// * topics - `["unpaused"]`
    /// * data - `[caller: Address]`
    fn unpause(e: Env, caller: Address);
}

// ################## ERRORS ##################

#[contracterror]
#[repr(u32)]
pub enum PausableError {
    /// The operation failed because the contract is paused.
    EnforcedPause = 1,
    /// The operation failed because the contract is not paused.
    ExpectedPause = 2,
}

// ################## EVENTS ##################

/// Emits an event when `Paused` state is triggered.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller.
///
/// # Events
///
/// * topics - `["paused"]`
/// * data - `[caller: Address]`
pub fn emit_paused(e: &Env, caller: &Address) {
    let topics = (symbol_short!("paused"),);
    e.events().publish(topics, caller)
}

/// Emits an event when `Unpaused` state is triggered.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `caller` - The address of the caller.
///
/// # Events
///
/// * topics - `["unpaused"]`
/// * data - `[caller: Address]`
pub fn emit_unpaused(e: &Env, caller: &Address) {
    let topics = (symbol_short!("unpaused"),);
    e.events().publish(topics, caller)
}
