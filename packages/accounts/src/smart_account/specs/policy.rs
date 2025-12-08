use soroban_sdk::{Address, Env, Val, Vec, auth::Context, contracttype};

use crate::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams},
        Policy,
        PolicyClientInterface,
    },
    smart_account::{ContextRule, Signer},
};

// NOTE: using a Simple Threshold policy for FV. Ideally we will want to verify with many different policies.
// Using a concrete one because we do need implementations of the methods.


pub struct SimpleThresholdPolicyContract;

impl SimpleThresholdPolicyContract {
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        crate::policies::simple_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    pub fn set_threshold(e: &Env, threshold: u32, context_rule: ContextRule, smart_account: Address) {
        crate::policies::simple_threshold::set_threshold(e, threshold, &context_rule, &smart_account)
    }
}

impl Policy for SimpleThresholdPolicyContract {
    type AccountParams = SimpleThresholdAccountParams;

    fn can_enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) -> bool {
        crate::policies::simple_threshold::can_enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) {
        crate::policies::simple_threshold::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn install(e: &Env, install_params: Self::AccountParams, context_rule: ContextRule, smart_account: Address) {
        crate::policies::simple_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        crate::policies::simple_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

