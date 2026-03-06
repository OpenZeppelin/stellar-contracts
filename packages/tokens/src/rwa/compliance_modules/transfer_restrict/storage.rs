use soroban_sdk::{contracttype, Address, Env};

use crate::rwa::compliance_modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

#[contracttype]
#[derive(Clone)]
pub enum TransferRestrictStorageKey {
    /// Per-(token, address) allowlist flag.
    AllowedUser(Address, Address),
}

/// Returns whether `user` is on the transfer allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The user address to check.
pub fn is_user_allowed(e: &Env, token: &Address, user: &Address) -> bool {
    let key = TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone());
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_: &bool| {
            e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        })
        .unwrap_or_default()
}

/// Adds `user` to the transfer allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The user address to allow.
pub fn set_user_allowed(e: &Env, token: &Address, user: &Address) {
    let key = TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone());
    e.storage().persistent().set(&key, &true);
    e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
}

/// Removes `user` from the transfer allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The user address to disallow.
pub fn remove_user_allowed(e: &Env, token: &Address, user: &Address) {
    e.storage()
        .persistent()
        .remove(&TransferRestrictStorageKey::AllowedUser(token.clone(), user.clone()));
}
