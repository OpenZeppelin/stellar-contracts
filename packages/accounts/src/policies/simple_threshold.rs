//! # Simple Threshold Policy Module
//!
//! This policy implements basic threshold functionality where a minimum number
//! of signers must be present for authorization, with all signers having equal
//! weight.
//!
//! # Security Warning: Signer Set Divergence
//!
//! This policy stores a threshold value that is validated against the number of
//! signers in a ContextRule at installation time. However, the policy is **NOT
//! automatically notified** when signers are added or removed from the
//! ContextRule. This creates a state divergence that can lead to:
//!
//! ## Denial of Service
//!
//! If signers are removed from the ContextRule after policy installation, the
//! total number of signers may fall below the stored threshold. This makes it
//! **impossible to meet the signature requirement**, permanently blocking any
//! actions governed by this policy until the threshold is manually updated.
//!
//! **Example:** A rule with 5 signers and threshold=5 (strict 5-of-5 multisig).
//! If 2 signers are removed, only 3 signers remain, making it impossible to
//! reach the threshold of 5.
//!
//! ## Unintentional Security Degradation
//!
//! If signers are added to the ContextRule after policy installation, the
//! security guarantee silently weakens without any explicit warning. A strict
//! N-of-N multisig becomes an N-of-(N+M) multisig, creating a false sense of
//! security.
//!
//! **Example:** A rule with 3 signers and threshold=3 (strict 3-of-3 multisig).
//! If 2 signers are added, it becomes a 3-of-5 multisig, meaning only 60% of
//! signers are required instead of 100%.
//!
//! ## Required Administrator Actions
//!
//! When modifying signers in a ContextRule with this policy:
//!
//! 1. **Review the current threshold** using `get_threshold()`
//! 2. **Calculate the new threshold** based on the desired security level
//! 3. **Update the threshold**, if necessary, with `set_threshold()` BEFORE
//!    removing or AFTER adding signers, ideally in the same transaction
//!
//! **Failure to follow this process may result in permanent DoS or silent
//! security degradation.**

use cvlr::nondet::Nondet;
use soroban_sdk::{
    auth::Context, contracterror, contracttype, panic_with_error, Address, Env, Vec,
};

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

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

impl Nondet for SimpleThresholdAccountParams {
    fn nondet() -> Self {
        SimpleThresholdAccountParams {
            threshold: u32::nondet(),
        }
    }
}

/// Error codes for simple threshold policy operations.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SimpleThresholdError {
    /// The smart account does not have a simple threshold policy installed.
    SmartAccountNotInstalled = 3200,
    /// When threshold is 0 or exceeds the number of available signers.
    InvalidThreshold = 3201,
    /// The transaction is not allowed by this policy.
    NotAllowed = 3202,
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
/// # Errors
///
/// * [`SimpleThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a simple threshold policy installed.
/// * [`SimpleThresholdError::NotAllowed`] - When threshold is not met.
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

    let threshold = get_threshold(e, context_rule.id, smart_account);

    if authenticated_signers.len() >= threshold {
        // emit event
        #[cfg(not(feature = "certora"))]
        SimplePolicyEnforced {
            smart_account: smart_account.clone(),
            context: context.clone(),
            context_rule_id: context_rule.id,
            authenticated_signers: authenticated_signers.clone(),
        }
        .publish(e);
    } else {
        panic_with_error!(e, SimpleThresholdError::NotAllowed)
    }
}

/// Sets the threshold value for a smart account's simple threshold policy.
/// Requires authorization from the smart account.
///
/// # Security Warning
///
/// **ALWAYS call this function BEFORE removing and AFTER adding signers** from
/// the ContextRule to maintain the desired security level and avoid DoS or
/// security degradation.
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
/// # Security Warning
///
/// After installation, the threshold is **NOT automatically updated** when
/// signers are added or removed from the ContextRule. Administrators must
/// manually call `set_threshold()` before or after modifying the signer set to
/// avoid DoS or security degradation. See module-level documentation for
/// details.
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
