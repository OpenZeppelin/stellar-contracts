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
//!     signer_weights: [(ceo_addr, 100), (cto_addr, 75), (cfo_addr, 75), (manager_addr, 50)],
//!     threshold: 150,
//! }
//! ```

use soroban_sdk::{
    auth::Context, contracterror, contracttype, panic_with_error, Address, Env, Map, Symbol, Vec,
};

// re-export
pub use crate::smart_account::Signer;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedThresholdInstallParams {
    pub signer_weights: Map<Signer, u32>,
    pub threshold: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum WeightedThresholdError {
    SmartAccountNotInstalled = 2210,
    InvalidWeight = 2211,
    InvalidThreshold = 2212,
    MathOverflow = 2213,
}

#[contracttype]
pub enum WeightedThresholdStorageKey {
    Threshold(Address),     // -> u32
    SignersWeight(Address), // -> Map(Signer, u32)
}

// ################## QUERY STATE ##################

pub fn get_threshold(e: &Env, smart_account: &Address) -> u32 {
    e.storage()
        .persistent()
        .get(&WeightedThresholdStorageKey::Threshold(smart_account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled))
}

pub fn get_signer_weights(e: &Env, smart_account: &Address) -> Map<Signer, u32> {
    e.storage()
        .persistent()
        .get(&WeightedThresholdStorageKey::SignersWeight(smart_account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled))
}

pub fn calculate_weight(e: &Env, signers: &Vec<Signer>, smart_account: &Address) -> u32 {
    let signer_weights = get_signer_weights(e, smart_account);

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

pub fn can_enforce(
    e: &Env,
    _context: &Context,
    _context_rule_signers: &Vec<Signer>,
    authenticated_signers: &Vec<Signer>,
    smart_account: &Address,
) -> bool {
    let threshold: Option<u32> = e
        .storage()
        .persistent()
        .get(&WeightedThresholdStorageKey::Threshold(smart_account.clone()));
    if let Some(threshold) = threshold {
        calculate_weight(e, authenticated_signers, smart_account) >= threshold
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

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
            (Symbol::new(e, "policy_enforced"), smart_account),
            (context.clone(), authenticated_signers.clone()),
        );
    }
}

pub fn set_threshold(e: &Env, threshold: u32, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if threshold == 0 {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold)
    }

    e.storage()
        .persistent()
        .set(&WeightedThresholdStorageKey::Threshold(smart_account.clone()), &threshold);
}

pub fn set_signer_weight(e: &Env, signer: &Signer, weight: u32, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    let key = WeightedThresholdStorageKey::SignersWeight(smart_account.clone());
    let mut signer_weights: Map<Signer, u32> =
        e.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled)
        });

    signer_weights.set(signer.clone(), weight);
    e.storage().persistent().set(&key, &signer_weights);
}

// can install only from the smart account
pub fn install(e: &Env, params: &WeightedThresholdInstallParams, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if params.threshold == 0 {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold)
    }

    let mut total_weight: u32 = 0;
    for weight in params.signer_weights.values() {
        if weight == 0 {
            panic_with_error!(e, WeightedThresholdError::InvalidWeight)
        }
        total_weight = total_weight
            .checked_add(weight)
            .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::MathOverflow));
    }

    if params.threshold > total_weight {
        panic_with_error!(e, WeightedThresholdError::InvalidThreshold);
    }

    e.storage()
        .persistent()
        .set(&WeightedThresholdStorageKey::Threshold(smart_account.clone()), &params.threshold);
    e.storage().persistent().set(
        &WeightedThresholdStorageKey::SignersWeight(smart_account.clone()),
        &params.signer_weights,
    );
}

// can uninstall only from the smart account
pub fn uninstall(e: &Env, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage().persistent().remove(&WeightedThresholdStorageKey::Threshold(smart_account.clone()));
    e.storage()
        .persistent()
        .remove(&WeightedThresholdStorageKey::SignersWeight(smart_account.clone()));
}
