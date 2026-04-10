use soroban_sdk::{contracttype, panic_with_error, vec, Address, Env, Vec};

use super::SupplyLimitSet;
use crate::rwa::compliance::{
    modules::{
        storage::{
            add_i128_or_panic, hooks_verified, require_non_negative_amount, sub_i128_or_panic,
            verify_required_hooks,
        },
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    ComplianceHook,
};

#[contracttype]
#[derive(Clone)]
pub enum SupplyLimitStorageKey {
    /// Per-token supply cap.
    SupplyLimit(Address),
    /// Per-token internal supply counter (updated via hooks).
    InternalSupply(Address),
}

// ################## RAW STORAGE ##################

/// Returns the supply limit for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_supply_limit(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Returns the supply limit for `token`, panicking if not configured.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::MissingLimit`] - When no supply limit has been
///   configured for this token.
pub fn get_supply_limit_or_panic(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    let limit: i128 = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::MissingLimit));
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
    limit
}

/// Sets the supply limit for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The maximum total supply.
pub fn set_supply_limit(e: &Env, token: &Address, limit: i128) {
    let key = SupplyLimitStorageKey::SupplyLimit(token.clone());
    e.storage().persistent().set(&key, &limit);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the internal supply counter for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_internal_supply(e: &Env, token: &Address) -> i128 {
    let key = SupplyLimitStorageKey::InternalSupply(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the internal supply counter for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `supply` - The new supply value.
pub fn set_internal_supply(e: &Env, token: &Address, supply: i128) {
    let key = SupplyLimitStorageKey::InternalSupply(token.clone());
    e.storage().persistent().set(&key, &supply);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

// ################## ACTIONS ##################

/// Validates, stores, and emits [`SupplyLimitSet`] for the given cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The supply cap.
pub fn configure_supply_limit(e: &Env, token: &Address, limit: i128) {
    require_non_negative_amount(e, limit);
    set_supply_limit(e, token, limit);
    SupplyLimitSet { token: token.clone(), limit }.publish(e);
}

/// Pre-seeds the internal supply counter for a token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `supply` - The pre-seeded supply value.
pub fn pre_set_supply(e: &Env, token: &Address, supply: i128) {
    require_non_negative_amount(e, supply);
    set_internal_supply(e, token, supply);
}

// ################## HOOK WIRING ##################

/// Returns the set of compliance hooks this module requires.
pub fn required_hooks(e: &Env) -> Vec<ComplianceHook> {
    vec![e, ComplianceHook::CanCreate, ComplianceHook::Created, ComplianceHook::Destroyed]
}

/// Cross-calls the compliance contract to verify that this module is
/// registered on all required hooks.
pub fn verify_hook_wiring(e: &Env) {
    verify_required_hooks(e, required_hooks(e));
}

// ################## COMPLIANCE HOOKS ##################

/// Updates the internal supply counter after a mint.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The minted amount.
/// * `token` - The token address.
pub fn on_created(e: &Env, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let current = get_internal_supply(e, token);
    set_internal_supply(e, token, add_i128_or_panic(e, current, amount));
}

/// Updates the internal supply counter after a burn.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The burned amount.
/// * `token` - The token address.
pub fn on_destroyed(e: &Env, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);
    let current = get_internal_supply(e, token);
    set_internal_supply(e, token, sub_i128_or_panic(e, current, amount));
}

/// Checks whether a mint would exceed the supply cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `amount` - The mint amount.
/// * `token` - The token address.
pub fn can_create(e: &Env, amount: i128, token: &Address) -> bool {
    assert!(
        hooks_verified(e),
        "SupplyLimitModule: not armed — call verify_hook_wiring() after wiring hooks [CanCreate, \
         Created, Destroyed]"
    );
    if amount < 0 {
        return false;
    }
    let limit = get_supply_limit(e, token);
    if limit == 0 {
        return true;
    }
    let supply = get_internal_supply(e, token);
    add_i128_or_panic(e, supply, amount) <= limit
}
