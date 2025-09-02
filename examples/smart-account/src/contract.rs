use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    Address, Env, String, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::{
    add_context_rule, get_context_rule, get_context_rules, modify_context_rule,
    remove_context_rule, storage::do_check_auth, ContextRule, ContextRuleType, ContextRuleVal,
    ExecutionEntry, Signatures, Signer, SmartAccount, SmartAccountError,
};

#[contract]
pub struct SmartAccountContract;

impl SmartAccountContract {
    pub fn __constructor(e: &Env, signers: Vec<Signer>, policies: Vec<Address>, name: String) {
        // TODO: validate signers.len() or policies.len()
        let context_rule_val = ContextRuleVal { name, signers, policies, valid_until: None };
        add_context_rule(e, &ContextRuleType::Default, &context_rule_val);
    }
}

#[contractimpl]
impl CustomAccountInterface for SmartAccountContract {
    type Error = SmartAccountError;
    type Signature = Signatures;

    fn __check_auth(
        e: Env,
        signature_payload: Hash<32>,
        signatures: Signatures,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        do_check_auth(e, signature_payload, signatures, auth_contexts)
    }
}

#[contractimpl]
impl SmartAccount for SmartAccountContract {
    type ContextRule = ContextRule;
    type ContextRuleType = ContextRuleType;
    type ContextRuleVal = ContextRuleVal;

    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule {
        get_context_rule(e, context_rule_id)
    }

    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule> {
        get_context_rules(e, &context_rule_type)
    }

    fn add_context_rule(
        e: &Env,
        context_rule_type: Self::ContextRuleType,
        context_rule_val: Self::ContextRuleVal,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        add_context_rule(e, &context_rule_type, &context_rule_val)
    }

    fn modify_context_rule(
        e: &Env,
        context_rule_id: u32,
        context_rule_val: Self::ContextRuleVal,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        modify_context_rule(e, context_rule_id, &context_rule_val)
    }

    fn remove_context_rule(e: &Env, context_rule_id: u32) {
        e.current_contract_address().require_auth();

        remove_context_rule(e, context_rule_id);
    }
}

#[contractimpl]
impl ExecutionEntry for SmartAccountContract {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();

        e.invoke_contract::<()>(&target, &target_fn, target_args);
    }
}
