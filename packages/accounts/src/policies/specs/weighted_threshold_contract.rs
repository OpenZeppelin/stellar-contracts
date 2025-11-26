use soroban_sdk::{contract, contractimpl, Env};
use crate::policies::Policy;
use crate::policies::weighted_threshold::WeightedThresholdAccountParams;
use crate::smart_account::{ContextRule, Signer};
use soroban_sdk::{auth::Context, Address, Vec, Map};
use crate::policies::weighted_threshold;

#[contract]
pub struct WeightedThresholdPolicy;

impl WeightedThresholdPolicy {
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        crate::policies::weighted_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    pub fn get_signer_weights(e: &Env, context_rule: ContextRule, smart_account: Address) -> Map<Signer, u32> {
        crate::policies::weighted_threshold::get_signer_weights(e, &context_rule, &smart_account)
    }

    pub fn set_threshold(e: &Env, threshold: u32, context_rule: ContextRule, smart_account: Address) {
        crate::policies::weighted_threshold::set_threshold(e, threshold, &context_rule, &smart_account)
    }

    pub fn set_signer_weight(e: &Env, signer: Signer, weight: u32, context_rule: ContextRule, smart_account: Address) {
        crate::policies::weighted_threshold::set_signer_weight(e, &signer, weight, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl Policy for WeightedThresholdPolicy {
    type AccountParams = WeightedThresholdAccountParams;

    fn can_enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) -> bool {
        crate::policies::weighted_threshold::can_enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) {
        crate::policies::weighted_threshold::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn install(e: &Env, install_params: Self::AccountParams, context_rule: ContextRule, smart_account: Address) {
        crate::policies::weighted_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        crate::policies::weighted_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

