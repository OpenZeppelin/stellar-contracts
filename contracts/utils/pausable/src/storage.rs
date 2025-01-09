use soroban_sdk::{panic_with_error, symbol_short, Address, Env, Symbol};

use crate::{emit_paused, emit_unpaused, pausable::PausableError};

/// Indicates whether the contract is in `Paused` state.
pub(crate) const PAUSED: Symbol = symbol_short!("PAUSED");

/// Returns true if the contract is paused, and false otherwise.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn paused(e: &Env) -> bool {
    // if not paused, consider default false (unpaused)
    e.storage().instance().get(&PAUSED).unwrap_or(false)
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
/// If the contract is in `Paused` state, then the error
/// [`PausableError::EnforcedPause`] is thrown.
///
/// # Events
///
/// * topics - `["paused"]`
/// * data - `[caller: Address]`
pub fn pause(e: &Env, caller: &Address) {
    caller.require_auth();
    when_not_paused(e);
    e.storage().instance().set(&PAUSED, &true);
    emit_paused(e, caller);
}

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
pub fn unpause(e: &Env, caller: &Address) {
    caller.require_auth();
    when_paused(e);
    e.storage().instance().set(&PAUSED, &false);
    emit_unpaused(e, caller);
}

/// Helper to make a function callable only when the contract is NOT
/// paused.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// If the contract is in the `Paused` state, then the error
/// [`PausableError::EnforcedPause`] is thrown.
pub fn when_not_paused(e: &Env) {
    if paused(e) {
        panic_with_error!(e, PausableError::EnforcedPause)
    }
}

/// Helper to make a function callable
/// only when the contract is paused.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// If the contract is in `Unpaused` state, then the error
/// [`PausableError::ExpectedPause`] is thrown.
pub fn when_paused(e: &Env) {
    if !paused(e) {
        panic_with_error!(e, PausableError::ExpectedPause)
    }
}
