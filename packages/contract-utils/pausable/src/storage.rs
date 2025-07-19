use soroban_sdk::{contracttype, panic_with_error, Env};

use crate::{emit_paused, emit_unpaused, pausable::{PausableError, Pausable}};

/// Storage key for the pausable state
#[contracttype]
pub enum PausableStorageKey {
    /// Indicates whether the contract is in paused state.
    Paused,
}

pub struct PausableDefault;

impl Pausable for PausableDefault {
    type Impl = Self;

    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn paused(e: &Env) -> bool {
        paused(e)
    }

    /// Triggers `Paused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn pause(e: &Env, _caller: &soroban_sdk::Address) {
        pause(e);
    }

    /// Triggers `Unpaused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn unpause(e: &Env, _caller: &soroban_sdk::Address) {
        unpause(e);
    }
}

/// Returns true if the contract is paused, and false otherwise.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
pub fn paused(e: &Env) -> bool {
    // if not paused, consider default false (unpaused)
    e.storage().instance().get(&PausableStorageKey::Paused).unwrap_or(false)

    // NOTE: We don't extend the TTL here. We don’t think utilities should
    // have any opinion on the TTLs, contracts usually manage TTL's themselves.
    // Extending the TTL in the utilities would be redundant in the most cases.
}

/// Triggers paused state.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * refer to [`when_not_paused`] errors.
///
/// # Events
///
/// * topics - `["paused"]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks and should only
/// be used in admin functions that implement their own authorization logic.
///
/// Example:
///
/// ```ignore,rust
/// use stellar_access_control_macros::only_role;
///
/// #[only_role(operator, "pauser")] // `only_role` handles authorization
/// fn emergency_pause(e: &Env, operator: Address) {
///     pausable::pause(e);
/// }
/// ```
pub fn pause(e: &Env) {
    when_not_paused(e);
    e.storage().instance().set(&PausableStorageKey::Paused, &true);
    emit_paused(e);
}

/// Triggers unpaused state.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * refer to [`when_paused`] errors.
///
/// # Events
///
/// * topics - `["unpaused"]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks and should only
/// be used in admin functions that implement their own authorization logic.
///
/// Example:
///
/// ```ignore,rust
/// use stellar_access_control_macros::only_role;
///
/// #[only_role(operator, "unpauser")] // `only_role` handles authorization
/// fn unpause(e: &Env, operator: Address) {
///     pausable::unpause(e);
/// }
/// ```
pub fn unpause(e: &Env) {
    when_paused(e);
    e.storage().instance().set(&PausableStorageKey::Paused, &false);
    emit_unpaused(e);
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
pub fn when_not_paused(e: &Env) {
    if paused(e) {
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
pub fn when_paused(e: &Env) {
    if !paused(e) {
        panic_with_error!(e, PausableError::ExpectedPause);
    }
}
