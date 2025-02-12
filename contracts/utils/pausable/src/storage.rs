use soroban_sdk::{panic_with_error, symbol_short, Address, Env, Symbol};

use crate::{emit_paused, emit_unpaused, pausable::PausableError};

/// Indicates whether the contract is in `Paused` state.
pub const PAUSED: Symbol = symbol_short!("PAUSED");

/// Returns true if the contract is paused, and false otherwise.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn paused(e: &Env) -> bool {
    // if not paused, consider default false (unpaused)
    e.storage().instance().get(&PAUSED).unwrap_or(false)

    // NOTE: We don't extend the TTL here. We don’t think utilities should
    // have any opinion on the TTLs, contracts usually manage TTL's themselves.
    // Extending the TTL in the utilities would be redundant in the most cases.
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
/// * [`PausableError::EnforcedPause`] - Occurs when the contract is already in
///   `Paused` state.
///
/// # Events
///
/// * topics - `["paused"]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// Authorization for `caller` is required.
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
/// * [`PausableError::ExpectedPause`] - Occurs when the contract is already in
///   `Unpaused` state.
///
/// # Events
///
/// * topics - `["unpaused"]`
/// * data - `[caller: Address]`
///
/// # Notes
///
/// Authorization for `caller` is required.
pub fn unpause(e: &Env, caller: &Address) {
    caller.require_auth();
    when_paused(e);
    e.storage().instance().set(&PAUSED, &false);
    emit_unpaused(e, caller);
}

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
///
/// # Notes
///
/// No authorization is required.
pub fn when_not_paused(e: &Env) {
    if paused(e) {
        panic_with_error!(e, PausableError::EnforcedPause)
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
///
/// # Notes
///
/// No authorization is required.
pub fn when_paused(e: &Env) {
    if !paused(e) {
        panic_with_error!(e, PausableError::ExpectedPause)
    }
}
