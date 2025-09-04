//! # Weighted Threshold Policy Module
//!
//! This policy implements weighted multisig functionality where different
//! signers have different voting weights, and a minimum total weight threshold
//! must be reached for authorization.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // CEO: weight 100, CTO: weight 75, CFO: weight 75, Manager: weight 25
//! // Threshold: 150 (requires CEO + one other, or CTO + CFO)
//! WeightedThresholdInstallParams {
//!     signer_weights: [(ceo_addr, 100), (cto_addr, 75), (cfo_addr, 75), (manager_addr, 25)],
//!     threshold: 150,
//! }
//! ```

use soroban_sdk::{
    auth::Context, contracterror, contracttype, panic_with_error, Address, Env, Map, Symbol, Vec,
};

// re-export
use crate::smart_account::{ContextRule, Signer};

/// Installation parameters for the weighted threshold policy.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedThresholdAccountParams {
    /// Mapping of signers to their respective weights.
    pub signer_weights: Map<Signer, u32>,
    /// The minimum total weight required for authorization.
    pub threshold: u32,
}

/// Error codes for weighted threshold policy operations.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum WeightedThresholdError {
    /// The smart account does not have a weighted threshold policy installed.
    SmartAccountNotInstalled = 2210,
    /// The threshold value is invalid.
    InvalidThreshold = 2211,
    /// A mathematical operation would overflow.
    MathOverflow = 2212,
}

/// Storage keys for weighted threshold policy data.
#[contracttype]
pub enum WeightedThresholdStorageKey {
    /// Storage key for the threshold value and signer weights of a smart
    /// account context rule. Maps to a `WeightedThresholdAccountParams`
    /// containing threshold and signer weights.
    AccountContext(Address, u32),
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const WEIGHTED_THRESHOLD_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const WEIGHTED_THRESHOLD_TTL_THRESHOLD: u32 = WEIGHTED_THRESHOLD_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## QUERY STATE ##################

/// Retrieves the threshold value for a smart account's weighted threshold
/// policy.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a weighted threshold policy installed.
pub fn get_threshold(e: &Env, context_rule: &ContextRule, smart_account: &Address) -> u32 {
    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let params: Option<WeightedThresholdAccountParams> =
        e.storage().persistent().get(&key).inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                WEIGHTED_THRESHOLD_TTL_THRESHOLD,
                WEIGHTED_THRESHOLD_EXTEND_AMOUNT,
            );
        });

    params
        .map(|p| p.threshold)
        .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled))
}

/// Retrieves the signer weights mapping for a smart account's weighted
/// threshold policy. Returns a map of signers to their respective weights.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a weighted threshold policy installed.
pub fn get_signer_weights(
    e: &Env,
    context_rule: &ContextRule,
    smart_account: &Address,
) -> Map<Signer, u32> {
    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let params: Option<WeightedThresholdAccountParams> =
        e.storage().persistent().get(&key).inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                WEIGHTED_THRESHOLD_TTL_THRESHOLD,
                WEIGHTED_THRESHOLD_EXTEND_AMOUNT,
            );
        });

    params
        .map(|p| p.signer_weights)
        .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled))
}

/// Calculates the total weight of the provided signers based on the smart
/// account's weighted threshold policy configuration. Returns the total weight
/// of all valid signers. Signers not in the policy configuration are ignored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The list of signers to calculate weight for.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::MathOverflow`] - When the total weight
///   calculation would overflow.
/// * [`WeightedThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a weighted threshold policy installed.
pub fn calculate_weight(
    e: &Env,
    signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) -> u32 {
    let signer_weights = get_signer_weights(e, context_rule, smart_account);

    let mut total_weight: u32 = 0;
    for signer in signers.iter() {
        // if no signer skip
        if let Some(weight) = signer_weights.get(signer.clone()) {
            total_weight = total_weight
                .checked_add(weight)
                .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::MathOverflow));
        }
    }
    total_weight
}

/// Checks if the weighted threshold policy can be enforced based on the total
/// weight of authenticated signers. Returns `true` if the total weight of
/// authenticated signers meets or exceeds the threshold, `false` otherwise or
/// if the policy is not installed.
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
    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let params: Option<WeightedThresholdAccountParams> = e.storage().persistent().get(&key);

    if let Some(params) = params {
        e.storage().persistent().extend_ttl(
            &key,
            WEIGHTED_THRESHOLD_TTL_THRESHOLD,
            WEIGHTED_THRESHOLD_EXTEND_AMOUNT,
        );
        calculate_weight(e, authenticated_signers, context_rule, smart_account) >= params.threshold
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

/// Enforces the weighted threshold policy if the weight requirements are met.
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
/// * topics - `["policy_enforced", smart_account: Address]`
/// * data - `[context: Context, authenticated_signers: Vec<Signer>]`
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
        e.events().publish(
            (Symbol::new(e, "policy_enforced"), smart_account),
            (context.clone(), authenticated_signers.clone()),
        );
    }
}

/// Sets the threshold value for a smart account's weighted threshold policy.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `threshold` - The minimum total weight required for authorization.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::InvalidThreshold`] - When threshold is 0.
/// * [`WeightedThresholdError::SmartAccountNotInstalled`] - When the policy is
///   not installed.
pub fn set_threshold(e: &Env, threshold: u32, context_rule: &ContextRule, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if threshold == 0 {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold)
    }

    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let mut params: WeightedThresholdAccountParams =
        e.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled)
        });

    params.threshold = threshold;

    // Check if threshold is reachable with current signer weights
    let total_weight = calculate_total_weight(e, &params.signer_weights);

    if threshold > total_weight {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold);
    }

    e.storage().persistent().set(&key, &params);
}

/// Sets the weight for a specific signer in the weighted threshold policy.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer` - The signer to set the weight for.
/// * `weight` - The weight value to assign to the signer.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::SmartAccountNotInstalled`] - When the smart
///   account does not have a weighted threshold policy installed.
pub fn set_signer_weight(
    e: &Env,
    signer: &Signer,
    weight: u32,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let mut params: WeightedThresholdAccountParams =
        e.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled)
        });

    params.signer_weights.set(signer.clone(), weight);

    // Check if threshold is still reachable with updated signer weights
    let total_weight = calculate_total_weight(e, &params.signer_weights);

    if params.threshold > total_weight {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold);
    }

    e.storage().persistent().set(&key, &params);
}

/// Installs the weighted threshold policy on a smart account.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - Installation parameters containing signer weights and
///   threshold.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`WeightedThresholdError::InvalidThreshold`] - When threshold is 0 or
///   exceeds the total weight of all signers.
/// * [`WeightedThresholdError::MathOverflow`] - When the total weight
///   calculation would overflow.
pub fn install(
    e: &Env,
    params: &WeightedThresholdAccountParams,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    let total_weight = calculate_total_weight(e, &params.signer_weights);

    if params.threshold == 0 || params.threshold > total_weight {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold);
    }

    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    e.storage().persistent().set(&key, params);
}

/// Uninstalls the weighted threshold policy from a smart account.
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

    let key = WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    e.storage().persistent().remove(&key);
}

/// Helper to calculate the total weight from a map of signer weights.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_weights` - Map of signers to their weights.
///
/// # Errors
///
/// * [`WeightedThresholdError::MathOverflow`] - When the total weight
///   calculation would overflow.
fn calculate_total_weight(e: &Env, signer_weights: &Map<Signer, u32>) -> u32 {
    let mut total_weight: u32 = 0;
    for weight in signer_weights.values() {
        total_weight = total_weight
            .checked_add(weight)
            .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::MathOverflow));
    }
    total_weight
}
