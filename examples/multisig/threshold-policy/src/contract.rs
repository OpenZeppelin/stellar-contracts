//! # Simple Threshold Policy Contract
//!
//! A reusable policy contract that implements simple threshold-based
//! authorization. This contract can be deployed once and used by multiple smart
//! accounts, with each account defining its own threshold value for different
//! context rules. Enables M-of-N multisig functionality where M signers out of
//! N total signers must authorize a transaction.
use soroban_sdk::{auth::Context, contract, contractimpl, Address, Env, Vec};
use stellar_accounts::{
    policies::{simple_threshold, Policy},
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct ThresholdPolicyContract;

#[contractimpl]
impl Policy for ThresholdPolicyContract {
    type AccountParams = simple_threshold::SimpleThresholdAccountParams;

    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool {
        simple_threshold::can_enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        )
    }

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        )
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        simple_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl ThresholdPolicyContract {
    /// Get the current threshold for a smart account
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        simple_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    /// Set a new threshold for a smart account
    pub fn set_threshold(
        e: Env,
        threshold: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::set_threshold(&e, threshold, &context_rule, &smart_account)
    }
}
