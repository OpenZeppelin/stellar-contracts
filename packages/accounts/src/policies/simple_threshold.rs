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

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SimpleThresholdInstallParams {
    pub threshold: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SimpleThresholdError {
    SmartAccountNotInstalled = 0,
    InvalidThreshold = 1,
}

#[contracttype]
pub enum SimpleThresholdStorageKey {
    Threshold(Address), // -> u32
}

// ################## QUERY STATE ##################

pub fn get_threshold(e: &Env, smart_account: &Address) -> u32 {
    e.storage()
        .persistent()
        .get(&SimpleThresholdStorageKey::Threshold(smart_account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, SimpleThresholdError::SmartAccountNotInstalled))
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
        authenticated_signers.len() >= threshold
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

// can install only from the smart account
pub fn install(e: &Env, params: &SimpleThresholdInstallParams, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if params.threshold == 0 {
        panic_with_error!(e, SimpleThresholdError::InvalidThreshold)
    }

    e.storage()
        .persistent()
        .set(&SimpleThresholdStorageKey::Threshold(smart_account.clone()), &params.threshold);
}

// can uninstall only from the smart account
pub fn uninstall(e: &Env, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage().persistent().remove(&SimpleThresholdStorageKey::Threshold(smart_account.clone()));
}
