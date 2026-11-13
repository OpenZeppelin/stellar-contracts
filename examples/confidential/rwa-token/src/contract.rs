//! # Confidential RWA Token Example
//!
//! A [`ConfidentialToken`] wired with the RWA profile of the compliance
//! extension (`packages/confidential/docs/compliance.md#rwa-profile`).
//! `type Hooks = RwaHooks` rejects every operation until the deployment is
//! configured, restricts accounts to the designated auditor, runs the freeze,
//! policy, and SAC gates on every party, and — when an approver is
//! configured — requires its authorization for every value-moving
//! operation. The contract implements [`ConfidentialCompliance`],
//! [`ConfidentialClawback`], [`ConfidentialRwa`], and
//! [`ConfidentialForcedTransfer`] on top.
//!
//! Wiring [`RwaHooks`] is what satisfies the deployment obligation of
//! [`ConfidentialClawback`] and [`ConfidentialForcedTransfer`]: a frozen
//! account cannot move its balance between the construction and the
//! submission of an override proof.
//!
//! Admin authority follows the separation of duties in
//! `packages/confidential/docs/compliance.md#admin-authority`, with one
//! `access_control` role per duty:
//!
//! | Role | Entry points |
//! |:---|:---|
//! | `freezer` | `freeze`, `unfreeze` |
//! | `config_manager` | `set_compliance_config`, `set_rwa_config`, `bump_approval_epoch` |
//! | `clawback` | `clawback`, `force_revoke_spender` |
//! | `forced_transfer` | `forced_transfer` |
//!
//! The constructor grants none of them; the admin grants each role after
//! deployment. The approver is not a role: it is the address set through
//! `set_rwa_config`, and it must not be the sender, recipient, or spender
//! of an operation it approves.
//!
//! `clawback` and `forced_transfer` take a proof whose witness includes the
//! auditor's secret key. Do not use either on a public network before the
//! verifier accepts zero-knowledge proofs
//! (`packages/confidential/docs/compliance.md#forced-transfer`).
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Symbol, Vec};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_confidential::{
    compliance::{
        storage as compliance, ClawbackData, ComplianceConfig, ConfidentialClawback,
        ConfidentialCompliance, ConfidentialForcedTransfer, ConfidentialRwa, RwaConfig, RwaHooks,
    },
    storage as confidential, ConfidentialAccount, ConfidentialToken, SpenderDelegation,
};
use stellar_macros::only_role;

#[contract]
pub struct RwaTokenContract;

#[contractimpl]
impl RwaTokenContract {
    /// Binds the token to its underlying asset and registries, sets the
    /// access-control admin, and writes both configurations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The access-control admin, which grants the `freezer`,
    ///   `config_manager`, `clawback`, and `forced_transfer` roles.
    /// * `underlying_asset` - The SEP-41 token backing confidential balances.
    /// * `verifier` - The confidential verifier registry.
    /// * `auditor` - The confidential auditor registry.
    /// * `compliance` - The initial compliance configuration. It must keep an
    ///   eligibility gate: a policy, SAC passthrough, or both.
    /// * `auditor_id` - The designated auditor every account binds to. It
    ///   cannot be changed afterwards.
    /// * `approver` - The address whose authorization every value-moving
    ///   operation additionally requires, or `None`.
    ///
    /// # Errors
    ///
    /// * refer to [`compliance::set_rwa_config`] errors.
    #[allow(clippy::too_many_arguments)]
    pub fn __constructor(
        e: &Env,
        admin: Address,
        underlying_asset: Address,
        verifier: Address,
        auditor: Address,
        compliance: ComplianceConfig,
        auditor_id: u32,
        approver: Option<Address>,
    ) {
        confidential::set_underlying_asset(e, &underlying_asset);
        confidential::set_verifier(e, &verifier);
        confidential::set_auditor(e, &auditor);
        confidential::set_address_as_field_element(e);
        access_control::set_admin(e, &admin);
        compliance::set_compliance_config(e, &compliance);
        compliance::set_rwa_config(e, auditor_id, &approver);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialToken for RwaTokenContract {
    type Hooks = RwaHooks;
}

#[contractimpl(contracttrait)]
impl ConfidentialCompliance for RwaTokenContract {
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
impl ConfidentialRwa for RwaTokenContract {
    #[only_role(operator, "config_manager")]
    fn set_rwa_config(e: &Env, auditor_id: u32, approver: Option<Address>, operator: Address) {
        compliance::set_rwa_config(e, auditor_id, &approver);
    }

    #[only_role(operator, "config_manager")]
    fn bump_approval_epoch(e: &Env, operator: Address) {
        compliance::bump_approval_epoch(e);
    }
}

#[contractimpl(contracttrait)]
impl ConfidentialClawback for RwaTokenContract {
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
impl ConfidentialForcedTransfer for RwaTokenContract {
    #[only_role(operator, "forced_transfer")]
    fn forced_transfer(
        e: &Env,
        from: Address,
        to: Address,
        amount: i128,
        data: Bytes,
        operator: Address,
    ) {
        let decoded: ClawbackData = confidential::decode_data(e, &data);
        compliance::forced_transfer(e, &from, &to, amount, &decoded.proof);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for RwaTokenContract {}
