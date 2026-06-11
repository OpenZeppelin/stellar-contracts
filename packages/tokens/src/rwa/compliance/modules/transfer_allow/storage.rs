use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::{
    modules::{
        transfer_allow::{emit_user_allowed, emit_user_disallowed},
        ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
    },
    TransferKind,
};

#[contracttype]
#[derive(Clone)]
pub enum TransferAllowStorageKey {
    /// Per-(token, user) allowlist membership entry.
    AllowedUser(Address, Address),
}

// ################## QUERY STATE ##################

/// Returns whether `user` is on the transfer allowlist for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to check.
pub fn is_user_allowed(e: &Env, token: &Address, user: &Address) -> bool {
    let key = TransferAllowStorageKey::AllowedUser(token.clone(), user.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Rejects a transfer where neither party, sender nor recipient, is on the
/// allowlist, by panicking.
///
/// The transfer amount has no effect on the decision. Forced
/// (admin/recovery) transfers are exempt from the policy, and no
/// bookkeeping exists in this module, so they pass through untouched.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `to` - The recipient address.
/// * `kind` - Who initiated the transfer and under what authority.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::UserNotAllowed`] - When neither party is on the
///   allowlist and the transfer is not forced.
pub fn on_transfer(e: &Env, from: &Address, to: &Address, kind: &TransferKind, token: &Address) {
    if *kind == TransferKind::Forced {
        return;
    }
    if !is_user_allowed(e, token, from) && !is_user_allowed(e, token, to) {
        panic_with_error!(e, ComplianceModuleError::UserNotAllowed);
    }
}

// ################## CHANGE STATE ##################

/// Adds `user` to the transfer allowlist for `token`. If `user` is already
/// allowed, the call is a no-op (no event emitted, no error raised).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to allow.
///
/// # Events
///
/// * topics - `["user_allowed", token: Address]`
/// * data - `[user: Address]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn allow_user(e: &Env, token: &Address, user: &Address) {
    if !is_user_allowed(e, token, user) {
        set_user_allowed(e, token, user);
        emit_user_allowed(e, token, user);
    }
}

/// Removes `user` from the transfer allowlist for `token`. If `user` is not
/// currently allowed, the call is a no-op (no event emitted, no error
/// raised).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to disallow.
///
/// # Events
///
/// * topics - `["user_disallowed", token: Address]`
/// * data - `[user: Address]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn disallow_user(e: &Env, token: &Address, user: &Address) {
    if is_user_allowed(e, token, user) {
        remove_user_allowed(e, token, user);
        emit_user_disallowed(e, token, user);
    }
}

/// Adds multiple users to the transfer allowlist in a single call. Entries
/// that are already allowed are silently skipped (no event emitted, no
/// error raised).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `users` - The addresses to allow.
///
/// # Events
///
/// For each user newly added to the allowlist:
/// * topics - `["user_allowed", token: Address]`
/// * data - `[user: Address]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
///
/// Each `(token, user)` pair lives in its own persistent entry, so the
/// caller must size `users` to stay within the per-transaction network
/// limits — see <https://lab.stellar.org/network-limits>.
pub fn batch_allow_users(e: &Env, token: &Address, users: &Vec<Address>) {
    for user in users.iter() {
        allow_user(e, token, &user);
    }
}

/// Removes multiple users from the transfer allowlist in a single call.
/// Entries that are not currently allowed are silently skipped (no event
/// emitted, no error raised).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `users` - The addresses to disallow.
///
/// # Events
///
/// For each user removed from the allowlist:
/// * topics - `["user_disallowed", token: Address]`
/// * data - `[user: Address]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
///
/// Each `(token, user)` pair lives in its own persistent entry, so the
/// caller must size `users` to stay within the per-transaction network
/// limits — see <https://lab.stellar.org/network-limits>.
pub fn batch_disallow_users(e: &Env, token: &Address, users: &Vec<Address>) {
    for user in users.iter() {
        disallow_user(e, token, &user);
    }
}

// ################## LOW-LEVEL HELPERS ##################

/// Records `user` as allowed in persistent storage, without the
/// existence check or the event of [`allow_user`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to allow.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_user_allowed(e: &Env, token: &Address, user: &Address) {
    let key = TransferAllowStorageKey::AllowedUser(token.clone(), user.clone());
    e.storage().persistent().set(&key, &());
}

/// Removes `user` from the allowlist in persistent storage, without the
/// existence check or the event of [`disallow_user`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `user` - The address to disallow.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_user_allowed(e: &Env, token: &Address, user: &Address) {
    e.storage()
        .persistent()
        .remove(&TransferAllowStorageKey::AllowedUser(token.clone(), user.clone()));
}
