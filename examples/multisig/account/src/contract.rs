use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl,
    crypto::Hash,
    panic_with_error, Address, Env, String, Symbol, Val, Vec,
};
use stellar_accounts::{
    policies::PolicyClient,
    smart_account::{
        add_context_rule, get_context_rule, get_context_rules, modify_context_rule,
        remove_context_rule, storage::do_check_auth, ContextRule, ContextRuleType, ContextRuleVal,
        ExecutionEntryPoint, Signatures, Signer, SmartAccount, SmartAccountError,
    },
};

#[contracterror]
#[repr(u32)]
pub enum MultisigError {
    NoSignersAndPolicies = 0,
}

#[contract]
pub struct MultisigContract;

impl MultisigContract {
    pub fn __constructor(
        e: &Env,
        signers: Vec<Signer>,
        policies: Vec<Address>,
        policies_install_params: Vec<Val>,
    ) {
        if signers.is_empty() && policies.is_empty() {
            panic_with_error!(e, MultisigError::NoSignersAndPolicies)
        }

        let context_rule_val = ContextRuleVal {
            name: String::from_str(e, "multisig"),
            signers,
            policies: policies.clone(),
            valid_until: None,
        };
        add_context_rule(e, &ContextRuleType::Default, &context_rule_val);

        Self::install_policies(e, policies, policies_install_params);
    }

    fn install_policies(e: &Env, policies: Vec<Address>, install_params: Vec<Val>) {
        for (policy, param) in policies.iter().zip(install_params.iter()) {
            PolicyClient::new(e, &policy).install(&param, &e.current_contract_address());
        }
    }

    //pub fn add_signer(e: &Env, context_rule_id: u32, signer: Signer) {
    //let rule = get_context_rule(e, context_rule_id);
    //}
}

#[contractimpl]
impl CustomAccountInterface for MultisigContract {
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
impl SmartAccount for MultisigContract {
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
impl ExecutionEntryPoint for MultisigContract {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();

        e.invoke_contract::<()>(&target, &target_fn, target_args);
    }
}
