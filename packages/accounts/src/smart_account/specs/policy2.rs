use soroban_sdk::{auth::Context, contracttype, Address, Env, Val, Vec};

use crate::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams},
        Policy, PolicyClientInterface,
    },
    smart_account::{ContextRule, Signer},
};

pub struct Policy2;

// TODO: implement this with ghosts
impl Policy2 {
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        crate::policies::simple_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    pub fn set_threshold(
        e: &Env,
        threshold: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        crate::policies::simple_threshold::set_threshold(
            e,
            threshold,
            &context_rule,
            &smart_account,
        )
    }
}

impl Policy for Policy2 {
    type AccountParams = SimpleThresholdAccountParams;

    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool {
        crate::policies::simple_threshold::can_enforce(
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
        crate::policies::simple_threshold::enforce(
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
        crate::policies::simple_threshold::install(
            e,
            &install_params,
            &context_rule,
            &smart_account,
        )
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        crate::policies::simple_threshold::uninstall(e, &context_rule, &smart_account)
    }
}
