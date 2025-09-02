//! # Simple Threshold Policy Module
//!
//! This policy implements basic threshold functionality where a minimum number
//! of signers must be present for authorization, with all signers having equal
//! weight.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // Require 2 out of 3 signers
//! SimpleThresholdInstallParams {
//!     signers: [addr1, addr2, addr3],
//!     threshold: 2,
//! }
//! ```

use soroban_sdk::{
    auth::Context, contracterror, contracttype, panic_with_error, Address, Env, Symbol, Vec,
};

// re-export
pub use crate::smart_account::Signer;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleThresholdInstallParams {
    pub signers: Vec<Signer>,
    pub threshold: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SimpleThresholdError {
    SmartAccountNotInstalled = 0,
    InvalidThreshold = 1,
    EmptySigners = 2,
    SignerAlreadyAdded = 3,
}

#[contracttype]
pub enum SimpleThresholdStorageKey {
    Threshold(Address), // -> u32
    Signers(Address),   // -> Vec<Signer>
}

// ################## QUERY STATE ##################

pub fn get_threshold(e: &Env, smart_account: &Address) -> u32 {
    e.storage()
        .persistent()
        .get(&SimpleThresholdStorageKey::Threshold(smart_account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled))
}

pub fn get_signers(e: &Env, smart_account: &Address) -> Vec<Signer> {
    e.storage()
        .persistent()
        .get(&SimpleThresholdStorageKey::Signers(smart_account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled))
}

pub fn count_authenticated_signers(
    e: &Env,
    authenticated_signers: &Vec<Signer>,
    smart_account: &Address,
) -> u32 {
    let policy_signers = get_signers(e, smart_account);

    let mut count = 0u32;
    for auth_signer in authenticated_signers.iter() {
        if policy_signers.contains(auth_signer) {
            count += 1;
        }
    }
    count
}

pub fn can_enforce(
    e: &Env,
    _context: &Context,
    _context_rule_signers: &Vec<Signer>,
    authenticated_signers: &Vec<Signer>,
    smart_account: &Address,
) -> bool {
    let threshold: Option<u32> =
        e.storage().persistent().get(&SimpleThresholdStorageKey::Threshold(smart_account.clone()));

    if let Some(threshold) = threshold {
        count_authenticated_signers(e, authenticated_signers, smart_account) >= threshold
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
            (Symbol::new(e, "simple_policy_enforced"), smart_account),
            (context.clone(), authenticated_signers.clone()),
        );
    }
}

pub fn set_threshold(e: &Env, threshold: u32, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if threshold == 0 {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Threshold(smart_account.clone()), &threshold);
}

pub fn add_signer(e: &Env, signer: &Signer, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    let key = SimpleThresholdStorageKey::Signers(smart_account.clone());
    let mut signers: Vec<Signer> =
        e.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled)
        });

    if !signers.contains(signer) {
        signers.push_back(signer.clone());
        e.storage().persistent().set(&key, &signers);
    } else {
        panic_with_error!(e, SimpleThresholdError::SignerAlreadyAdded)
    }
}

pub fn remove_signer(e: &Env, signer: &Signer, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    let key = SimpleThresholdStorageKey::Signers(smart_account.clone());
    let mut signers: Vec<Signer> =
        e.storage().persistent().get(&key).unwrap_or_else(|| {
            panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled)
        });

    if let Some(pos) = signers.iter().position(|s| s == *signer) {
        signers.remove(pos as u32);
        e.storage().persistent().set(&key, &signers);
    }
}

// can install only from the smart account
pub fn install(e: &Env, params: &SimpleThresholdInstallParams, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if params.threshold == 0 {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    if params.signers.is_empty() {
        panic_with_error!(e, SimpleThresholdError::EmptySigners)
    }

    if params.threshold > params.signers.len() {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold);
    }

    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Threshold(smart_account.clone()), &params.threshold);
    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Signers(smart_account.clone()), &params.signers);
}

// can uninstall only from the smart account
pub fn uninstall(e: &Env, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage().persistent().remove(&SimpleThresholdStorageKey::Threshold(smart_account.clone()));
    e.storage().persistent().remove(&SimpleThresholdStorageKey::Signers(smart_account.clone()));
}
