use soroban_sdk::{contracterror, symbol_short, Address, Env};

pub trait Pausable {
    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn paused(e: &Env) -> bool {
        crate::paused(e)
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
    /// * data - `[caller: Address]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::pause()`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// IMPORTANT: The base implementation of [`crate::pause()`] intentionally
    /// lacks authorization controls. If you want to restrict who can
    /// `pause` the contract, you MUST implement proper authorization in
    /// your contract.
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
    /// * data - `[caller: Address]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::unpause()`] when implementing this function.
    ///
    /// # Security Warning
    ///
    /// IMPORTANT: The base implementation of [`crate::unpause()`] intentionally
    /// lacks authorization controls. If you want to restrict who can
    /// `unpause` the contract, you MUST implement proper authorization in
    /// your contract.
    fn unpause(e: &Env, caller: Address);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PausableError {
    /// The operation failed because the contract is paused.
    EnforcedPause = 100,
    /// The operation failed because the contract is not paused.
    ExpectedPause = 101,
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
