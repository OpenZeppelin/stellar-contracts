//! # Simple Threshold Policy Module
//!
//! This policy implements basic threshold functionality where a minimum number
//! of signers must be present for authorization, with all signers having equal
//! weight.

use soroban_sdk::{
    auth::Context, contracterror, contracttype, panic_with_error, Address, Env, Symbol, Vec,
};

// re-export
pub use crate::smart_account::Signer;

/// Installation parameters for the simple threshold policy.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleThresholdInstallParams {
    /// The minimum number of signers required for authorization.
    pub threshold: u32,
    /// The total number of signers available for this policy.
    pub signers_count: u32,
}

/// Error codes for simple threshold policy operations.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SimpleThresholdError {
    /// The smart account does not have a simple threshold policy installed.
    SmartAccountNotInstalled = 2200,
    /// When threshold is 0 or exceeds the number of available signers.
    InvalidThreshold = 2201,
}

/// Storage keys for simple threshold policy data.
#[contracttype]
pub enum SimpleThresholdStorageKey {
    /// Storage key for the threshold value of a smart account.
    /// Maps to a `u32` representing the minimum number of signers required.
    Threshold(Address),
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const SIMPLE_THRESHOLD_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const SIMPLE_THRESHOLD_TTL_THRESHOLD: u32 = SIMPLE_THRESHOLD_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## QUERY STATE ##################

/// Retrieves the threshold value for a smart account's simple threshold policy.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a simple threshold policy installed.
pub fn get_threshold(e: &Env, smart_account: &Address) -> u32 {
    let key = SimpleThresholdStorageKey::Threshold(smart_account.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                SIMPLE_THRESHOLD_TTL_THRESHOLD,
                SIMPLE_THRESHOLD_EXTEND_AMOUNT,
            );
        })
        .unwrap_or_else(|| panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled))
}

/// Checks if the simple threshold policy can be enforced based on the number
/// of authenticated signers. Returns `true` if the number of authenticated
/// signers meets or exceeds the threshold, `false` otherwise or if the policy
/// is not installed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `_context` - The authorization context (unused).
/// * `_context_rule_signers` - The signers from context rules (unused).
/// * `authenticated_signers` - The list of authenticated signers.
/// * `smart_account` - The address of the smart account.
pub fn can_enforce(
    e: &Env,
    _context: &Context,
    _context_rule_signers: &Vec<Signer>,
    authenticated_signers: &Vec<Signer>,
    smart_account: &Address,
) -> bool {
    let key = SimpleThresholdStorageKey::Threshold(smart_account.clone());
    let threshold: Option<u32> = e.storage().persistent().get(&key);

    if let Some(threshold) = threshold {
        e.storage().persistent().extend_ttl(
            &key,
            SIMPLE_THRESHOLD_TTL_THRESHOLD,
            SIMPLE_THRESHOLD_EXTEND_AMOUNT,
        );
        authenticated_signers.len() >= threshold
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

/// Enforces the simple threshold policy if the threshold requirements are met.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `context_rule_signers` - The signers from the context rule.
/// * `authenticated_signers` - The list of authenticated signers.
/// * `smart_account` - The address of the smart account.
///
/// # Events
///
/// * topics - `["simple_policy_enforced", smart_account: Address]`
/// * data - `[context: Context, authenticated_signers: Vec<Signer>]`
pub fn enforce(
    e: &Env,
    context: &Context,
    context_rule_signers: &Vec<Signer>,
    authenticated_signers: &Vec<Signer>,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if can_enforce(e, context, context_rule_signers, authenticated_signers, smart_account) {
        // emit event
        e.events().publish(
            (Symbol::new(e, "simple_policy_enforced"), smart_account),
            (context.clone(), authenticated_signers.clone()),
        );
    }
}

/// Sets the threshold value for a smart account's simple threshold policy.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum number of signers required.
/// * `signers_count` - The total number of signers available.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total number of signers.
pub fn set_threshold(e: &Env, threshold: u32, signers_count: u32, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if threshold == 0 || threshold > signers_count {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Threshold(smart_account.clone()), &threshold);
}

/// Installs the simple threshold policy on a smart account.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - Installation parameters containing threshold and signers count.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total number of signers.
pub fn install(e: &Env, params: &SimpleThresholdInstallParams, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if params.threshold == 0 || params.threshold > params.signers_count {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Threshold(smart_account.clone()), &params.threshold);
}

/// Uninstalls the simple threshold policy from a smart account.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
pub fn uninstall(e: &Env, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage().persistent().remove(&SimpleThresholdStorageKey::Threshold(smart_account.clone()));
}
