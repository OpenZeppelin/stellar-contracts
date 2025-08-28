use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    Address, Env, FromVal, String, Symbol, Val, Vec,
};
use storage::{
    add_context_rule, authenticate, enforce_policy, get_context_rule, get_context_rules,
    get_validated_context, ContextRule, ContextRuleType, ContextRuleVal, Signatures, Signer,
    SmartAccountError,
};

pub mod storage;
mod test;

// provide user defined types to generalize the interface
pub trait SmartAccount {
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
    );

    // TODO
    //fn remove_context_rule(e: &Env, context_rule_id: u32);

    // TODO
    //fn modify_context_rule(e: &Env, context_rule_id: u32, context_rule_val:
    // Self::ContextRuleVal);

    // do we need those at all, given there's `modify_context_rule`
    //fn add_signer(e: &Env, signer: Signer, context_rule_id: u32);
    //fn remove_signer(e: &Env, signer: Signer, context_rule_id: u32);
}

// Simple execution entry-point to call arbitrary contracts.
//
// Most likely to be used to call owned
// stateful polciies, and as direct contract-to-contract invocations are always
// authorized, that's a way to avoid re-entry when the policy need to auth back
// its owner.
pub trait ExecutationEntry: CustomAccountInterface {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}

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
        authenticate(&e, &signature_payload, &signatures);

        let mut validated_contexts = Vec::new(&e);
        for context in auth_contexts.iter() {
            validated_contexts.push_back(get_validated_context(&e, &context, &signatures.0.keys()));
        }

        // after collecting validated context rules and authenticated signers, call for
        // every policy `PolicyClient::on_enforce` to trigger the state-changing
        // effects if any
        for (rule, context, authenticated_signers) in validated_contexts.iter() {
            let ContextRule { signers, policies, .. } = rule;
            for policy in policies.iter() {
                enforce_policy(&e, &policy, &context, &signers, &authenticated_signers);
            }
        }

        Ok(())
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
    ) {
        e.current_contract_address().require_auth();

        add_context_rule(e, &context_rule_type, &context_rule_val);
    }
}

#[contractimpl]
impl ExecutationEntry for SmartAccountContract {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();

        e.invoke_contract::<()>(&target, &target_fn, target_args);
    }
}
