pub mod storage;
mod test;
use soroban_sdk::{auth::CustomAccountInterface, Address, Env, FromVal, Symbol, Val, Vec};
pub use storage::{
    add_context_rule, authenticate, enforce_policy, get_context_rule, get_context_rules,
    get_validated_context, modify_context_rule, remove_context_rule, ContextRule, ContextRuleType,
    ContextRuleVal, Signatures, Signer, SmartAccountError,
};

// provide user defined types to generalize the interface
pub trait SmartAccount: CustomAccountInterface {
    type ContextRule: FromVal<Env, Val>;
    type ContextRuleType: FromVal<Env, Val>;
    type ContextRuleVal: FromVal<Env, Val>;

    fn get_context_rule(e: &Env, context_rule_id: u32) -> Self::ContextRule;

    fn get_context_rules(
        e: &Env,
        context_rule_type: Self::ContextRuleType,
    ) -> Vec<Self::ContextRule>;

    fn add_context_rule(
        e: &Env,
        context_rule_type: Self::ContextRuleType,
        context_rule_val: Self::ContextRuleVal,
    ) -> Self::ContextRule;

    fn modify_context_rule(
        e: &Env,
        context_rule_id: u32,
        context_rule_val: Self::ContextRuleVal,
    ) -> Self::ContextRule;

    fn remove_context_rule(e: &Env, context_rule_id: u32);
}

// Simple execution entry-point to call arbitrary contracts.
//
// Most likely to be used to call owned stateful polciies, and as direct
// contract-to-contract invocations are always authorized, that's a way to avoid
// re-entry when the policy need to auth back its owner.
pub trait ExecutionEntryPoint {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}
