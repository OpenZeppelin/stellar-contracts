use soroban_sdk::{contracttype, Address, Env};

use crate::rwa::compliance_modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

#[contracttype]
#[derive(Clone)]
pub enum MaxBalanceStorageKey {
    /// Per-token maximum allowed identity balance.
    MaxBalance(Address),
    /// Balance keyed by (token, identity) — not by wallet.
    IDBalance(Address, Address),
}

/// Returns the per-identity balance cap for `token`, or `0` if not set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_max_balance(e: &Env, token: &Address) -> i128 {
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the per-identity balance cap for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `value` - The maximum balance per identity.
pub fn set_max_balance(e: &Env, token: &Address, value: i128) {
    let key = MaxBalanceStorageKey::MaxBalance(token.clone());
    e.storage().persistent().set(&key, &value);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Returns the tracked balance for `identity` on `token`, or `0` if not
/// set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
pub fn get_id_balance(e: &Env, token: &Address, identity: &Address) -> i128 {
    let key = MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &i128| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Sets the tracked balance for `identity` on `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The on-chain identity address.
/// * `balance` - The new balance value.
pub fn set_id_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    let key = MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone());
    e.storage().persistent().set(&key, &balance);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}
