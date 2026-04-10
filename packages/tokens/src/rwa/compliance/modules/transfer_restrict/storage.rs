use soroban_sdk::{contracttype, Address, Env, Vec};

use super::{UserAllowed, UserDisallowed};
use crate::rwa::compliance::modules::{MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD};

#[contracttype]
#[derive(Clone)]
pub enum TransferRestrictStorageKey {
    /// Per-(token, address) allowlist flag.
    AllowedUser(Address, Address),
}

// ################## RAW STORAGE ##################

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

// ################## ACTIONS ##################

/// Adds `user` to the transfer allowlist for `token` and emits
/// [`UserAllowed`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to allow.
pub fn allow_user(e: &Env, token: &Address, user: &Address) {
    set_user_allowed(e, token, user);
    UserAllowed { token: token.clone(), user: user.clone() }.publish(e);
}

/// Removes `user` from the transfer allowlist for `token` and emits
/// [`UserDisallowed`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to disallow.
pub fn disallow_user(e: &Env, token: &Address, user: &Address) {
    remove_user_allowed(e, token, user);
    UserDisallowed { token: token.clone(), user: user.clone() }.publish(e);
}

/// Adds multiple users to the transfer allowlist in a single call.
/// Emits [`UserAllowed`] for each user added.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `users` - The addresses to allow.
pub fn batch_allow_users(e: &Env, token: &Address, users: &Vec<Address>) {
    for user in users.iter() {
        set_user_allowed(e, token, &user);
        UserAllowed { token: token.clone(), user }.publish(e);
    }
}

/// Removes multiple users from the transfer allowlist in a single call.
/// Emits [`UserDisallowed`] for each user removed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `users` - The addresses to disallow.
pub fn batch_disallow_users(e: &Env, token: &Address, users: &Vec<Address>) {
    for user in users.iter() {
        remove_user_allowed(e, token, &user);
        UserDisallowed { token: token.clone(), user }.publish(e);
    }
}

// ################## COMPLIANCE HOOKS ##################

/// Checks whether the transfer is allowed by the address allowlist.
///
/// T-REX semantics: if the sender is allowlisted, the transfer passes;
/// otherwise the recipient must be allowlisted.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `token` - The token address.
///
/// # Returns
///
/// `true` if the sender or recipient is allowlisted, `false` otherwise.
pub fn can_transfer(e: &Env, from: &Address, to: &Address, token: &Address) -> bool {
    if is_user_allowed(e, token, from) {
        return true;
    }
    is_user_allowed(e, token, to)
}
