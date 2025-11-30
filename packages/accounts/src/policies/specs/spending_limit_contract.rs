use soroban_sdk::{contract, contractimpl, Env};
use crate::policies::Policy;
use crate::policies::spending_limit::SpendingLimitAccountParams;
use crate::policies::spending_limit::SpendingLimitData;
use crate::smart_account::{ContextRule, Signer};
use soroban_sdk::{auth::Context, Address, Vec};
use crate::policies::spending_limit;

#[contract]
pub struct SpendingLimitPolicy;

impl SpendingLimitPolicy {
    pub fn get_spending_limit_data(e: &Env, context_rule_id: u32, smart_account: Address) -> SpendingLimitData {
        crate::policies::spending_limit::get_spending_limit_data(e, context_rule_id, &smart_account)
    }

    pub fn set_spending_limit(e: &Env, spending_limit: i128, context_rule: ContextRule, smart_account: Address) {
        crate::policies::spending_limit::set_spending_limit(e, spending_limit, &context_rule, &smart_account)
    }
}

// #[contractimpl] -- doesn't compile with this because of duplicate names of contract functions in the same crate.
impl Policy for SpendingLimitPolicy {
    type AccountParams = SpendingLimitAccountParams;

    fn can_enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) -> bool {
        crate::policies::spending_limit::can_enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn enforce(e: &Env, context: Context, authenticated_signers: Vec<Signer>, context_rule: ContextRule, smart_account: Address) {
        crate::policies::spending_limit::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    fn install(e: &Env, install_params: Self::AccountParams, context_rule: ContextRule, smart_account: Address) {
        crate::policies::spending_limit::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        crate::policies::spending_limit::uninstall(e, &context_rule, &smart_account)
    }
}

