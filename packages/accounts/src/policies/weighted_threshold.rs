//! # Weighted Threshold Policy
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
//! WeightedThresholdPolicy {
//!     signer_weights: [(ceo_addr, 100), (cto_addr, 75), (cfo_addr, 75), (manager_addr, 25)],
//!     threshold: 150,
//! }
//! ```

use soroban_sdk::{
    auth::Context, contract, contracterror, contractimpl, contracttype, panic_with_error, Address,
    Env, Map, Vec,
};

use crate::{policies::Policy, smart_account::storage::Signer};

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum WeightedThresholdError {
    InsufficientWeight = 1,
    ConfigNotFound = 2,
    InvalidWeight = 3,
    InvalidThreshold = 4,
    WrongSmartAccount = 5,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedThresholdConfig {
    pub signer_weights: Map<Signer, u32>, // Weight for each signer
    pub threshold: u32,                   // Minimum total weight required
}

#[contracttype]
pub enum WeightedThresholdStorageKey {
    Config,
    SmartAccount,
}

#[contract]
pub struct WeightedThresholdPolicy;

#[contractimpl]
impl WeightedThresholdPolicy {
    pub fn __constructor(
        e: &Env,
        signer_weights: Map<Signer, u32>,
        threshold: u32,
        smart_account: Address,
    ) {
        if threshold == 0 {
            panic_with_error!(e, WeightedThresholdError::InvalidThreshold)
        }

        let mut total_weight: u32 = 0;
        for weight in signer_weights.values() {
            if weight == 0 {
                panic_with_error!(e, WeightedThresholdError::InvalidWeight)
            }
            // TODO: check overflow
            total_weight += weight;
        }

        if threshold > total_weight {
            panic_with_error!(e, WeightedThresholdError::InvalidThreshold);
        }

        let config = WeightedThresholdConfig { signer_weights, threshold };

        e.storage().persistent().set(&WeightedThresholdStorageKey::Config, &config);
        e.storage().persistent().set(&WeightedThresholdStorageKey::SmartAccount, &smart_account);
    }

    /// Get the current configuration
    pub fn get_config(e: Env) -> WeightedThresholdConfig {
        e.storage()
            .persistent()
            .get(&WeightedThresholdStorageKey::Config)
            .unwrap_or_else(|| panic_with_error!(&e, WeightedThresholdError::ConfigNotFound))
    }

    /// Calculate total weight for a set of signers
    pub fn calculate_weight(e: Env, signers: Vec<Signer>) -> u32 {
        let config = Self::get_config(e);
        let mut total_weight: u32 = 0;

        for signer in signers.iter() {
            if let Some(weight) = config.signer_weights.get(signer.clone()) {
                total_weight += weight;
            }
        }

        total_weight
    }
}

#[contractimpl]
impl Policy for WeightedThresholdPolicy {
    /// Check if the weighted multisig policy can be enforced
    /// Returns true if the authenticated signers have sufficient combined
    /// weight
    fn can_enforce(
        e: &Env,
        _source: Address,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
    ) -> bool {
        let config =
            e.storage().persistent().get::<WeightedThresholdStorageKey, WeightedThresholdConfig>(
                &WeightedThresholdStorageKey::Config,
            );

        if let Some(config) = config {
            let mut total_weight: u32 = 0;

            for signer in authenticated_signers.iter() {
                if let Some(weight) = config.signer_weights.get(signer.clone()) {
                    total_weight += weight;
                }
            }

            total_weight >= config.threshold
        } else {
            false // No configuration found
        }
    }

    /// Enforce the weighted multisig policy
    /// This implementation is stateless but could track enforcement history
    fn on_enforce(
        e: &Env,
        source: Address,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
    ) {
        // Require authorization from the source (smart account)
        source.require_auth();
        let smart_account = e
            .storage()
            .persistent()
            .get(&WeightedThresholdStorageKey::SmartAccount)
            .expect("smart account is set");
        if source != smart_account {
            panic_with_error!(e, WeightedThresholdError::WrongSmartAccount)
        }

        let config = e
            .storage()
            .persistent()
            .get::<WeightedThresholdStorageKey, WeightedThresholdConfig>(
                &WeightedThresholdStorageKey::Config,
            )
            .unwrap_or_else(|| panic_with_error!(e, WeightedThresholdError::ConfigNotFound));

        let mut total_weight: u32 = 0;

        for signer in authenticated_signers.iter() {
            if let Some(weight) = config.signer_weights.get(signer.clone()) {
                total_weight += weight;
            }
        }

        if total_weight < config.threshold {
            panic_with_error!(e, WeightedThresholdError::InsufficientWeight);
        }
    }
}
