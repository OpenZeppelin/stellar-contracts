//! # Social Recovery Policy Contract
//!
//! A reusable policy contract that lets a quorum of guardians restore access
//! to a smart account whose owner lost their keys. This contract can be
//! deployed once and used by any number of smart accounts, each configuring
//! its own guardians, threshold, delay and expiry per recovery rule.
//!
//! The contract has no constructor, admin or upgrade entry point, so its code
//! cannot change once deployed. An upgradeable deployment would let its
//! upgrader recover every smart account that uses it.
use soroban_sdk::{auth::Context, contract, contractimpl, Address, BytesN, Env, Vec};
use stellar_accounts::{
    policies::{
        social_recovery::{self, RecoveryAttempt, SocialRecoveryAccountParams},
        Policy,
    },
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct SocialRecoveryPolicyContract;

#[contractimpl]
impl Policy for SocialRecoveryPolicyContract {
    type AccountParams = SocialRecoveryAccountParams;

    /// Enforce the social recovery policy.
    ///
    /// Admits only the completion of an executable recovery attempt, then
    /// retires the attempt and emits an event.
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        social_recovery::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    /// Install the social recovery policy for a smart account.
    ///
    /// Stores the recovery configuration for the given context rule.
    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        social_recovery::install(e, &install_params, &context_rule, &smart_account)
    }

    /// Uninstall the social recovery policy for a smart account.
    ///
    /// Removes the recovery configuration for the given context rule.
    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        social_recovery::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl SocialRecoveryPolicyContract {
    /// Approve a signer set as a guardian of a smart account's recovery rule.
    pub fn approve(
        e: &Env,
        guardian: Address,
        epoch: u32,
        signers: Vec<Signer>,
        context_rule_id: u32,
        smart_account: Address,
    ) {
        social_recovery::approve(e, &guardian, epoch, &signers, context_rule_id, &smart_account)
    }

    /// Revoke a guardian's approval of a signer set.
    pub fn revoke_approval(
        e: &Env,
        guardian: Address,
        epoch: u32,
        signers: Vec<Signer>,
        context_rule_id: u32,
        smart_account: Address,
    ) {
        social_recovery::revoke_approval(
            e,
            &guardian,
            epoch,
            &signers,
            context_rule_id,
            &smart_account,
        )
    }

    /// Cancel every open recovery attempt of a smart account's recovery rule.
    pub fn cancel(e: &Env, context_rule_id: u32, smart_account: Address) {
        social_recovery::cancel(e, context_rule_id, &smart_account)
    }

    /// Replace the recovery configuration of a smart account's recovery rule.
    pub fn set_config(
        e: &Env,
        params: SocialRecoveryAccountParams,
        context_rule_id: u32,
        smart_account: Address,
    ) {
        social_recovery::set_config(e, &params, context_rule_id, &smart_account)
    }

    /// Get the recovery configuration of a smart account's recovery rule.
    pub fn get_config(
        e: &Env,
        context_rule_id: u32,
        smart_account: Address,
    ) -> SocialRecoveryAccountParams {
        social_recovery::get_config(e, context_rule_id, &smart_account)
    }

    /// Get the current epoch of a smart account's recovery rule.
    pub fn get_epoch(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        social_recovery::get_epoch(e, context_rule_id, &smart_account)
    }

    /// Get the recovery attempt for a signer set at an epoch.
    pub fn get_attempt(
        e: &Env,
        epoch: u32,
        signers_hash: BytesN<32>,
        context_rule_id: u32,
        smart_account: Address,
    ) -> Option<RecoveryAttempt> {
        social_recovery::get_attempt(e, epoch, &signers_hash, context_rule_id, &smart_account)
    }
}
