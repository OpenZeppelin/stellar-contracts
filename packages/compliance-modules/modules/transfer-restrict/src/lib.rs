#![no_std]
//! Transfer restriction (address allowlist) compliance module — Stellar port
//! of T-REX [`TransferRestrictModule.sol`][trex-src].
//!
//! Maintains a per-token address allowlist. Transfers pass if the sender is
//! on the list; otherwise the recipient must be. No identity resolution is
//! needed — this module operates purely on wallet addresses.
//!
//! ## Hook mapping (T-REX → Stellar)
//!
//! | T-REX hook             | Stellar hook    | Behaviour                                      |
//! |------------------------|-----------------|-------------------------------------------------|
//! | `moduleCheck`          | `can_transfer`  | Pass if sender allowed, else require recipient |
//! | _(same)_               | `can_create`    | Always true (mints bypass allowlist)           |
//! | `moduleTransferAction` | `on_transfer`   | No-op                                          |
//! | `moduleMintAction`     | `on_created`    | No-op                                          |
//! | `moduleBurnAction`     | `on_destroyed`  | No-op                                          |
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/TransferRestrictModule.sol

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env, Vec};

use stellar_tokens::rwa::compliance::ComplianceModule;

use stellar_compliance_common::{
    get_compliance_address, module_name, require_compliance_auth, set_compliance_address,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Per-(token, address) allowlist flag.
    AllowedUser(Address, Address),
}

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

/// Restricts transfers to a per-address allowlist. Passes if sender or
/// recipient is on the list.
#[contract]
pub struct TransferRestrictModule;

#[contractimpl]
impl TransferRestrictModule {
    /// Adds `user` to the transfer allowlist for `token`.
    pub fn allow_user(e: &Env, token: Address, user: Address) {
        require_compliance_auth(e);
        e.storage().persistent().set(&DataKey::AllowedUser(token.clone(), user.clone()), &true);
        UserAllowed { token, user }.publish(e);
    }

    /// Removes `user` from the transfer allowlist for `token`.
    pub fn disallow_user(e: &Env, token: Address, user: Address) {
        require_compliance_auth(e);
        e.storage().persistent().remove(&DataKey::AllowedUser(token.clone(), user.clone()));
        UserDisallowed { token, user }.publish(e);
    }

    /// Adds multiple addresses to the transfer allowlist in a single call.
    pub fn batch_allow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_compliance_auth(e);
        for user in users.iter() {
            e.storage().persistent().set(&DataKey::AllowedUser(token.clone(), user.clone()), &true);
            UserAllowed { token: token.clone(), user }.publish(e);
        }
    }

    /// Removes multiple addresses from the transfer allowlist in a single call.
    pub fn batch_disallow_users(e: &Env, token: Address, users: Vec<Address>) {
        require_compliance_auth(e);
        for user in users.iter() {
            e.storage().persistent().remove(&DataKey::AllowedUser(token.clone(), user.clone()));
            UserDisallowed { token: token.clone(), user }.publish(e);
        }
    }

    /// Returns `true` if `user` is on the allowlist for `token`.
    pub fn is_user_allowed(e: &Env, token: Address, user: Address) -> bool {
        e.storage().persistent().get(&DataKey::AllowedUser(token, user)).unwrap_or_default()
    }
}

#[contractimpl]
impl ComplianceModule for TransferRestrictModule {
    /// No-op — stateless module.
    fn on_transfer(_e: &Env, _from: Address, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_created(_e: &Env, _to: Address, _amount: i128, _token: Address) {}

    /// No-op — stateless module.
    fn on_destroyed(_e: &Env, _from: Address, _amount: i128, _token: Address) {}

    /// Returns `true` if sender or recipient is on the allowlist.
    fn can_transfer(e: &Env, from: Address, to: Address, _amount: i128, token: Address) -> bool {
        // T-REX semantics: if sender is allowlisted, transfer passes; otherwise
        // recipient must be allowlisted.
        if Self::is_user_allowed(e, token.clone(), from) {
            return true;
        }
        Self::is_user_allowed(e, token, to)
    }

    /// Always returns `true` — mints bypass the allowlist check.
    fn can_create(_e: &Env, _to: Address, _amount: i128, _token: Address) -> bool {
        true
    }

    fn name(e: &Env) -> soroban_sdk::String {
        module_name(e, "TransferRestrictModule")
    }

    fn get_compliance_address(e: &Env) -> Address {
        get_compliance_address(e)
    }

    fn set_compliance_address(e: &Env, compliance: Address) {
        set_compliance_address(e, &compliance);
    }
}
