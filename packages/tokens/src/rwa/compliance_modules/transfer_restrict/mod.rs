//! Transfer restriction (address allowlist) compliance module — Stellar port
//! of T-REX [`TransferRestrictModule.sol`][trex-src].
//!
//! Maintains a per-token address allowlist. Transfers pass if the sender is
//! on the list; otherwise the recipient must be.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TransferRestrictModule.sol

pub mod storage;

use soroban_sdk::{contractevent, contracttrait, Address, Env, String, Vec};
use storage::{is_user_allowed, remove_user_allowed, set_user_allowed};

use super::common::{
    get_compliance_address, module_name, require_compliance_auth, set_compliance_address,
};

/// Emitted when an address is added to the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAllowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}

/// Emitted when an address is removed from the transfer allowlist.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserDisallowed {
    #[topic]
    pub token: Address,
    pub user: Address,
}

/// Transfer restriction compliance trait.
///
/// Provides default implementations for maintaining a per-token address
/// allowlist. Transfers are allowed if the sender is allowlisted; otherwise
/// the recipient must be (T-REX semantics).
#[contracttrait]
pub trait TransferRestrict {
    /// Adds `user` to the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `user` - The address to allow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`UserAllowed`].
    fn allow_user(e: &Env, token: Address, user: Address) {
        require_compliance_auth(e);
        set_user_allowed(e, &token, &user);
        UserAllowed { token, user }.publish(e);
    }

    /// Removes `user` from the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `user` - The address to disallow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`UserDisallowed`].
    fn disallow_user(e: &Env, token: Address, user: Address) {
        require_compliance_auth(e);
        remove_user_allowed(e, &token, &user);
        UserDisallowed { token, user }.publish(e);
    }

    /// Adds multiple users to the transfer allowlist in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `users` - The addresses to allow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`UserAllowed`] for each user added.
    fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_compliance_auth(e);
        for user in users.iter() {
            set_user_allowed(e, &token, &user);
            UserAllowed { token: token.clone(), user }.publish(e);
        }
    }

    /// Removes multiple users from the transfer allowlist in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `users` - The addresses to disallow.
    ///
    /// # Authorization
    ///
    /// Requires compliance contract authorization.
    ///
    /// # Events
    ///
    /// Emits [`UserDisallowed`] for each user removed.
    fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_compliance_auth(e);
        for user in users.iter() {
            remove_user_allowed(e, &token, &user);
            UserDisallowed { token: token.clone(), user }.publish(e);
        }
    }

    /// Returns whether `user` is on the transfer allowlist for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `user` - The address to check.
    fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
        is_user_allowed(e, &token, &user)
    }

    /// No-op — this module does not track transfer state.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track mint state.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — this module does not track burn state.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

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
    /// * `_amount` - The transfer amount (unused).
    /// * `token` - The token address.
    ///
    /// # Returns
    ///
    /// `true` if the sender or recipient is allowlisted, `false` otherwise.
    fn can_transfer(e: &Env, from: Address, to: Address, _amount: i128, token: Address) -> bool {
        if is_user_allowed(e, &token, &from) {
            return true;
        }
        is_user_allowed(e, &token, &to)
    }

    /// Always returns `true` — mints are not restricted by this module.
    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    /// Returns the module name for identification.
    fn name(e: &Env) -> String {
        module_name(e, "TransferRestrictModule")
    }

    /// Returns the compliance contract address.
    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    /// Sets the compliance contract address (one-time only).
    ///
    /// # Panics
    ///
    /// Panics if the compliance address has already been set.
    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
