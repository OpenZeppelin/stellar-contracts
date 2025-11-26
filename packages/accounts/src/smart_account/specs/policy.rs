use soroban_sdk::{Address, Env, Val, Vec, contracttype};

use crate::{
    policies::{
        simple_threshold::{self, SimpleThresholdAccountParams},
        Policy,
        PolicyClientInterface,
    },
    smart_account::{ContextRule, Signer},
};

// NOTE: dummy policy template. Please implement as needed before using this.


pub struct PolicyContract;

// NOTE: change this as needed.
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct Params {
    pub field1: u32,
    pub field2: i128,
}

impl Policy for PolicyContract {
    type AccountParams = Params;

    fn can_enforce(
        e: &Env,
        context: soroban_sdk::auth::Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool {
        // Implement
        true
    }

    fn enforce(
        e: &Env,
        context: soroban_sdk::auth::Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        // Implement
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        // Implement
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        // Implement
    }
}