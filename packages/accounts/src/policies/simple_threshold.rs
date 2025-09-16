//! # Simple Threshold Policy Module
//!
//! This policy implements basic threshold functionality where a minimum number
//! of signers must be present for authorization, with all signers having equal
//! weight.

use soroban_sdk::{
    auth::Context, contracterror, contractevent, contracttype, panic_with_error, Address, Env, Vec,
};

use crate::smart_account::ContextRule;
// re-export
pub use crate::smart_account::Signer;

/// Event emitted when a simple threshold policy is enforced.
#[contractevent]
#[derive(Clone)]
pub struct SimplePolicyEnforced {
    #[topic]
    pub smart_account: Address,
    pub context: Context,
    pub context_rule_id: u32,
    pub authenticated_signers: Vec<Signer>,
}

/// Installation parameters for the simple threshold policy.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleThresholdAccountParams {
    /// The minimum number of signers required for authorization.
    pub threshold: u32,
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
    AccountContext(Address, u32),
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
/// * `context_rule_id` - The context rule ID for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a simple threshold policy installed.
pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: &Address) -> u32 {
    let key = SimpleThresholdStorageKey::AccountContext(smart_account.clone(), context_rule_id);
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
/// * `authenticated_signers` - The list of authenticated signers.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
pub fn can_enforce(
    e: &Env,
    _context: &Context,
    authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) -> bool {
    let key = SimpleThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
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
/// * `authenticated_signers` - The list of authenticated signers.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Events
///
/// * topics - `["simple_policy_enforced", smart_account: Address]`
/// * data - `[context: Context, context_rule_id: u32 authenticated_signers:
///   Vec<Signer>]`
pub fn enforce(
    e: &Env,
    context: &Context,
    authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if can_enforce(e, context, authenticated_signers, context_rule, smart_account) {
        // emit event
        SimplePolicyEnforced {
            smart_account: smart_account.clone(),
            context: context.clone(),
            context_rule_id: context_rule.id,
            authenticated_signers: authenticated_signers.clone(),
        }
        .publish(e);
    }
}

/// Sets the threshold value for a smart account's simple threshold policy.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum number of signers required for authorization.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total number of signers.
pub fn set_threshold(e: &Env, threshold: u32, context_rule: &ContextRule, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    validate_and_set_threshold(e, threshold, context_rule, smart_account);
}

/// Installs the simple threshold policy on a smart account.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - Installation parameters containing the threshold.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total number of signers in the context rule.
pub fn install(
    e: &Env,
    params: &SimpleThresholdAccountParams,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    validate_and_set_threshold(e, params.threshold, context_rule, smart_account);
}

/// Uninstalls the simple threshold policy from a smart account.
/// Removes all stored threshold data for the account and context rule.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
pub fn uninstall(e: &Env, context_rule: &ContextRule, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage()
        .persistent()
        .remove(&SimpleThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id));
}

/// Internal function that validates and sets the threshold.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum number of signers required for authorization.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SimpleThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total number of signers.
fn validate_and_set_threshold(
    e: &Env,
    threshold: u32,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    if threshold == 0 || threshold > context_rule.signers.len() {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    e.storage().persistent().set(
        &SimpleThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id),
        &threshold,
    );
}
