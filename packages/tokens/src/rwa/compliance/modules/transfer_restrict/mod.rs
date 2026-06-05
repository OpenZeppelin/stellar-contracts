//! Transfer-restriction compliance module — Stellar port of T-REX
//! [`TransferRestrictModule.sol`][trex-src].
//!
//! Maintains a per-token address allowlist. A transfer passes when the
//! sender is allowlisted; otherwise the recipient must be allowlisted.
//! Mints and burns are not restricted by this module.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TransferRestrictModule.sol

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Transfer Restriction Compliance Module Trait
///
/// The `TransferRestrict` trait extends the [`ComplianceModule`] trait to
/// provide a per-token address allowlist. When this module is registered on
/// a token's modular compliance contract, transfers are permitted only when
/// the sender is on the allowlist or, failing that, the recipient is. A
/// typical use case is restricting secondary trading to a known set of
/// venues: allowlisting an exchange lets anyone trade with it, while
/// transfers between two unlisted wallets stay blocked.
///
/// Mints and burns are intentionally not restricted.
///
/// Unlike identity-based modules (such as the country allowlist), this
/// module checks wallet addresses directly and does not consult the
/// Identity Registry Storage. The allowlist is tracked per-`token`, so a
/// single module contract can serve multiple tokens with independent
/// allowlists.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
#[contracttrait]
pub trait TransferRestrict: ComplianceModule {
    /// Adds `user` to the transfer allowlist for `token`. If `user` is
    /// already allowed, the call is a no-op (no event emitted, no error
    /// raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `user` - The address to allow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::allow_user`] for the
    /// implementation.
    fn allow_user(e: &Env, token: Address, user: Address, operator: Address);

    /// Removes `user` from the transfer allowlist for `token`. If `user` is
    /// not currently allowed, the call is a no-op (no event emitted, no
    /// error raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `user` - The address to disallow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::disallow_user`] for
    /// the implementation.
    fn disallow_user(e: &Env, token: Address, user: Address, operator: Address);

    /// Adds multiple users to the transfer allowlist for `token`. Entries
    /// that are already allowed are silently skipped (no event emitted, no
    /// error raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `users` - The addresses to allow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each user newly added to the allowlist:
    /// * topics - `["user_allowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::batch_allow_users`]
    /// for the implementation.
    ///
    /// Each `(token, user)` pair is stored in its own persistent entry, so
    /// the caller must size `users` to stay within the per-transaction
    /// network limits — see <https://lab.stellar.org/network-limits>.
    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>, operator: Address);

    /// Removes multiple users from the transfer allowlist for `token`.
    /// Entries that are not currently allowed are silently skipped (no event
    /// emitted, no error raised).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose allowlist is updated.
    /// * `users` - The addresses to disallow.
    /// * `operator` - The address authorized to perform this operation.
    ///
    /// # Events
    ///
    /// For each user removed from the allowlist:
    /// * topics - `["user_disallowed", token: Address]`
    /// * data - `[user: Address]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced on `operator` before calling [`storage::batch_disallow_users`]
    /// for the implementation.
    ///
    /// Each `(token, user)` pair lives in its own persistent entry, so the
    /// caller must size `users` to stay within the per-transaction network
    /// limits — see <https://lab.stellar.org/network-limits>.
    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>, operator: Address);

    /// Returns `true` if `user` is on the transfer allowlist for `token`.
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
