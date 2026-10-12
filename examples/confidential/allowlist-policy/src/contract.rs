//! # Confidential Allowlist Policy Example
//!
//! A standalone [`Policy`] registry for the confidential token's compliance
//! extension: `is_authorized` returns `true` only for accounts a `manager` has
//! listed. Membership state, its TTL extension, and the `UserAllowed` /
//! `UserDisallowed` events come from [`AllowList`].
//!
//! `is_authorized` ignores its `token` argument, so one registry serves every
//! token whose `ComplianceConfig::policy` points at it. A deployment that needs
//! per-token rules, such as a jurisdiction filter, reads `token` there.
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_confidential::compliance::Policy;
use stellar_macros::only_role;
use stellar_tokens::fungible::allowlist::AllowList;

const MANAGER_ROLE: Symbol = symbol_short!("manager");

#[contract]
pub struct AllowlistPolicyContract;

#[contractimpl]
impl AllowlistPolicyContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address) {
        access_control::set_admin(e, &admin);
        access_control::grant_role_no_auth(e, &manager, &MANAGER_ROLE, &admin);
    }

    /// Lists `account`. A no-op when `account` is already listed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to list.
    /// * `operator` - The `manager` role holder authorizing the call.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", user: Address]`
    /// * data - `[]`
    #[only_role(operator, "manager")]
    pub fn allow(e: &Env, account: Address, operator: Address) {
        AllowList::allow_user(e, &account);
    }

    /// Unlists `account`. A no-op when `account` is not listed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to unlist.
    /// * `operator` - The `manager` role holder authorizing the call.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", user: Address]`
    /// * data - `[]`
    #[only_role(operator, "manager")]
    pub fn disallow(e: &Env, account: Address, operator: Address) {
        AllowList::disallow_user(e, &account);
    }

    /// Returns whether `account` is listed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check.
    pub fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }
}

#[contractimpl]
impl Policy for AllowlistPolicyContract {
    fn is_authorized(e: Env, account: Address, _token: Address) -> bool {
        AllowList::allowed(&e, &account)
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for AllowlistPolicyContract {}
