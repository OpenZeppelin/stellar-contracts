//! # Compliant Confidential Token Example
//!
//! A [`ConfidentialToken`] wired with the compliance extension. `type Hooks =
//! ComplianceHooks` runs the freeze, policy, and SAC gates at every entry
//! point, and the contract implements [`ConfidentialCompliance`] and
//! [`ConfidentialClawback`] on top.
//!
//! Wiring [`ComplianceHooks`] is what satisfies the deployment obligation in
//! the [`ConfidentialClawback`] trait-level security warning: a frozen account
//! cannot move its balance between the construction and the submission of a
//! clawback proof.
//!
//! Admin authority follows the separation of duties in
//! `packages/confidential/docs/compliance.md#admin-authority`, with one
//! `access_control` role per duty:
//!
//! | Role | Entry points |
//! |:---|:---|
//! | `freezer` | `freeze`, `unfreeze` |
//! | `config_manager` | `set_compliance_config` |
//! | `clawback` | `clawback`, `force_revoke_spender` |
//!
//! The constructor grants none of them; the admin grants each role after
//! deployment.
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, MuxedAddress, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_confidential::{
    compliance::{
        storage as compliance, ClawbackData, ComplianceConfig, ComplianceHooks,
        ConfidentialClawback, ConfidentialCompliance,
    },
    storage as confidential, ConfidentialAccount, ConfidentialToken, SpenderDelegation,
};
use stellar_macros::only_role;

#[contract]
pub struct CompliantTokenContract;

#[contractimpl]
impl CompliantTokenContract {
    /// Binds the token to its underlying asset and registries, sets the
    /// access-control admin, and writes the initial compliance configuration.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The access-control admin, which grants the `freezer`,
    ///   `config_manager`, and `clawback` roles.
    /// * `underlying_asset` - The SEP-41 token backing confidential balances.
    /// * `verifier` - The confidential verifier registry.
    /// * `auditor` - The confidential auditor registry.
    /// * `compliance` - The initial configuration. With `sac_passthrough` set,
    ///   every gated operation calls `authorized` on `underlying_asset`, which
    ///   traps unless it is a Stellar Asset Contract.
    pub fn __constructor(
        e: &Env,
        admin: Address,
        underlying_asset: Address,
        verifier: Address,
        auditor: Address,
        compliance: ComplianceConfig,
    ) {
        confidential::set_underlying_asset(e, &underlying_asset);
        confidential::set_verifier(e, &verifier);
        confidential::set_auditor(e, &auditor);
        confidential::set_address_as_field_element(e);
        access_control::set_admin(e, &admin);
        compliance::set_compliance_config(e, &compliance);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialToken for CompliantTokenContract {
    type Hooks = ComplianceHooks;
}

#[contractimpl(contracttrait)]
impl ConfidentialCompliance for CompliantTokenContract {
    #[only_role(operator, "freezer")]
    fn freeze(e: &Env, account: Address, operator: Address) {
        compliance::freeze(e, &account);
    }

    #[only_role(operator, "freezer")]
    fn unfreeze(e: &Env, account: Address, operator: Address) {
        compliance::unfreeze(e, &account);
    }

    #[only_role(operator, "config_manager")]
    fn set_compliance_config(e: &Env, config: ComplianceConfig, operator: Address) {
        compliance::set_compliance_config(e, &config);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialClawback for CompliantTokenContract {
    #[only_role(operator, "clawback")]
    fn clawback(
        e: &Env,
        account: Address,
        amount: i128,
        destination: Option<Address>,
        data: Bytes,
        operator: Address,
    ) {
        let decoded: ClawbackData = confidential::decode_data(e, &data);
        compliance::clawback(e, &account, amount, &destination, &decoded.proof);
    }

    #[only_role(operator, "clawback")]
    fn force_revoke_spender(e: &Env, account: Address, spender: Address, operator: Address) {
        compliance::force_revoke_spender(e, &account, &spender);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for CompliantTokenContract {}
