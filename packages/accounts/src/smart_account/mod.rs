pub mod storage;
//mod test;
use soroban_sdk::{auth::CustomAccountInterface, Address, Env, Map, String, Symbol, Val, Vec};
pub use storage::{
    add_context_rule, add_policy, add_signer, authenticate, get_context_rule, get_context_rules,
    get_validated_context, remove_context_rule, remove_policy, remove_signer,
    update_context_rule_name, update_context_rule_valid_until, ContextRule, ContextRuleType, Meta,
    Signatures, Signer, SmartAccountError,
};

pub trait SmartAccount: CustomAccountInterface {
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule;

    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule>;

    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule;

    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule;

    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule;

    fn remove_context_rule(e: &Env, context_rule_id: u32);

    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer);

    fn remove_signer(e: &Env, context_rule_id: u32, signer: Signer);

    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val);

    fn remove_policy(e: &Env, context_rule_id: u32, policy: Address);
}

// Simple execution entry-point to call arbitrary contracts.
//
// Most likely to be used to call owned stateful polciies, and as direct
// contract-to-contract invocations are always authorized, that's a way to avoid
// re-entry when the policy need to auth back its owner.
pub trait ExecutionEntryPoint {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}
