//! Transfer restriction (address allowlist) compliance module — Stellar port
//! of T-REX [`TransferRestrictModule.sol`][trex-src].
//!
//! Maintains a per-token address allowlist. Transfers pass if the sender is
//! on the list; otherwise the recipient must be.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TransferRestrictModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Transfer restriction compliance module trait.
///
/// This trait defines the contract-facing API for the transfer restriction
/// module. Low-level state changes live in [`storage`]. Privileged methods have
/// no default implementation because each contract must enforce its own access
/// control before delegating to storage helpers.
#[contracttrait]
pub trait TransferRestrict: ComplianceModule {
    /// Adds `user` to the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `user` - The address to allow.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn allow_user(e: &Env, token: Address, user: Address);

    /// Removes `user` from the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `user` - The address to disallow.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn disallow_user(e: &Env, token: Address, user: Address);

    /// Adds multiple users to the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `users` - The addresses to allow.
    ///
    /// # Events
    ///
    /// For each user newly added:
    /// * topics - `["user_allowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>);

    /// Removes multiple users from the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `users` - The addresses to disallow.
    ///
    /// # Events
    ///
    /// For each user removed:
    /// * topics - `["user_disallowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control.
    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>);

    /// Returns whether `user` is on the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is queried.
    /// * `user` - The address to check.
    fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
        storage::is_user_allowed(e, &token, &user)
    }
}

// ################## EVENTS ##################

/// Emitted when an address is added to the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAllowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}

/// Emits a [`UserAllowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `user` - The address that was allowed.
pub fn emit_user_allowed(e: &Env, token: &Address, user: &Address) {
    UserAllowed { token: token.clone(), user: user.clone() }.publish(e);
}

/// Emitted when an address is removed from the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserDisallowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}

/// Emits a [`UserDisallowed`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose allowlist changed.
/// * `user` - The address that was disallowed.
pub fn emit_user_disallowed(e: &Env, token: &Address, user: &Address) {
    UserDisallowed { token: token.clone(), user: user.clone() }.publish(e);
}
