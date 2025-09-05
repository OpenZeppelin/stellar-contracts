use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    Address, Env, Map, String, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::{
    add_context_rule, add_policy, add_signer, do_check_auth, get_context_rule, get_context_rules,
    remove_context_rule, remove_policy, remove_signer, update_context_rule_name,
    update_context_rule_valid_until, ContextRule, ContextRuleType, ExecutionEntryPoint, Signatures,
    Signer, SmartAccount, SmartAccountError,
};

#[contract]
pub struct MultisigContract;

impl MultisigContract {
    pub fn __constructor(e: &Env, signers: Vec<Signer>, policies: Map<Address, Val>) {
        add_context_rule(
            e,
            &ContextRuleType::Default,
            &String::from_str(e, "multisig"),
            None,
            &signers,
            &policies,
        );
    }
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
        do_check_auth(&e, &signature_payload, &signatures, &auth_contexts)
    }
}

#[contractimpl]
impl SmartAccount for MultisigContract {
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule {
        get_context_rule(e, context_rule_id)
    }

    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule> {
        get_context_rules(e, &context_rule_type)
    }

    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        add_context_rule(e, &context_type, &name, valid_until, &signers, &policies)
    }

    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule {
        e.current_contract_address().require_auth();

        update_context_rule_name(e, context_rule_id, &name)
    }

    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        update_context_rule_valid_until(e, context_rule_id, valid_until)
    }

    fn remove_context_rule(e: &Env, context_rule_id: u32) {
        e.current_contract_address().require_auth();

        remove_context_rule(e, context_rule_id);
    }

    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer) {
        e.current_contract_address().require_auth();

        add_signer(e, context_rule_id, &signer);
    }

    fn remove_signer(e: &Env, context_rule_id: u32, signer: Signer) {
        e.current_contract_address().require_auth();

        remove_signer(e, context_rule_id, &signer);
    }

    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val) {
        e.current_contract_address().require_auth();

        add_policy(e, context_rule_id, &policy, install_param);
    }

    fn remove_policy(e: &Env, context_rule_id: u32, policy: Address) {
        e.current_contract_address().require_auth();

        remove_policy(e, context_rule_id, &policy);
    }
}

#[contractimpl]
impl ExecutionEntryPoint for MultisigContract {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();

        e.invoke_contract::<()>(&target, &target_fn, target_args);
    }
}
