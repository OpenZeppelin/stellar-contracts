//! # Smart Account Example - Multisig
//!
//! A core smart account contract implementation that demonstrates the use of
//! context rules, signers, and policies. This contract can be configured as
//! a multisig by using the simple threshold policy, or customized with other
//! policies for different authorization patterns. This contract is upgradeable.
use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contractimpl,
    crypto::Hash,
    Address, BytesN, Env, Map, String, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::{
    self, AuthPayload, ContextRule, ContextRuleType, ExecutionEntryPoint, Signer, SmartAccount,
    SmartAccountError,
};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};

#[contract]
pub struct MultisigContract;

#[contractimpl]
impl MultisigContract {
    /// Creates a default context rule with the provided signers and policies.
    ///
    /// # Arguments
    ///
    /// * `signers` - Vector of signers (Delegated or External) that can
    ///   authorize transactions
    /// * `policies` - Map of policy contract addresses to their installation
    ///   parameters
    pub fn __constructor(e: &Env, signers: Vec<Signer>, policies: Map<Address, Val>) {
        smart_account::add_context_rule(
            e,
            &ContextRuleType::Default,
            &String::from_str(e, "multisig"),
            None,
            &signers,
            &policies,
        );
    }

    pub fn batch_add_signer(e: &Env, context_rule_id: u32, signers: Vec<Signer>) {
        e.current_contract_address().require_auth();

        smart_account::batch_add_signer(e, context_rule_id, &signers);
    }
}

#[contractimpl]
impl CustomAccountInterface for MultisigContract {
    type Error = SmartAccountError;
    type Signature = AuthPayload;

    /// Verify authorization for the smart account.
    ///
    /// This function is called by the Soroban host when authorization is
    /// required. It validates signatures against the configured context
    /// rules and policies.
    ///
    /// # Arguments
    ///
    /// * `signature_payload` - Hash of the data that was signed
    /// * `signatures` - Map of signers to their signature data
    /// * `auth_contexts` - Contexts being authorized (contract calls,
    ///   deployments, etc.)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if authorization succeeds
    /// * `Err(SmartAccountError)` if authorization fails
    fn __check_auth(
        e: Env,
        signature_payload: Hash<32>,
        signatures: AuthPayload,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        smart_account::do_check_auth(&e, &signature_payload, &signatures, &auth_contexts)
    }
}

#[contractimpl]
impl SmartAccount for MultisigContract {
    /// Retrieve a specific context rule by its ID.
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule {
        smart_account::get_context_rule(e, context_rule_id)
    }

    /// Retrieve the number of all context rules, including the expired ones.
    fn get_context_rules_count(e: &Env) -> u32 {
        smart_account::get_context_rules_count(e)
    }

    /// Add a new context rule to the smart account.
    ///
    /// Requires smart account authorization.
    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        smart_account::add_context_rule(e, &context_type, &name, valid_until, &signers, &policies)
    }

    /// Update the name of an existing context rule.
    ///
    /// Requires smart account authorization.
    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule {
        e.current_contract_address().require_auth();

        smart_account::update_context_rule_name(e, context_rule_id, &name)
    }

    /// Update the expiration time of an existing context rule.
    ///
    /// Requires smart account authorization.
    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        smart_account::update_context_rule_valid_until(e, context_rule_id, valid_until)
    }

    /// Remove a context rule from the smart account.
    ///
    /// Requires smart account authorization.
    fn remove_context_rule(e: &Env, context_rule_id: u32) {
        e.current_contract_address().require_auth();

        smart_account::remove_context_rule(e, context_rule_id);
    }

    /// Add a signer to an existing context rule.
    ///
    /// Requires smart account authorization.
    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer) -> u32 {
        e.current_contract_address().require_auth();

        smart_account::add_signer(e, context_rule_id, &signer)
    }

    /// Remove a signer from an existing context rule.
    ///
    /// Requires smart account authorization.
    fn remove_signer(e: &Env, context_rule_id: u32, signer_id: u32) {
        e.current_contract_address().require_auth();

        smart_account::remove_signer(e, context_rule_id, signer_id);
    }

    /// Add a policy to an existing context rule.
    ///
    /// Requires smart account authorization.
    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val) -> u32 {
        e.current_contract_address().require_auth();

        smart_account::add_policy(e, context_rule_id, &policy, install_param)
    }

    /// Remove a policy from an existing context rule.
    ///
    /// Requires smart account authorization.
    fn remove_policy(e: &Env, context_rule_id: u32, policy_id: u32) {
        e.current_contract_address().require_auth();

        smart_account::remove_policy(e, context_rule_id, policy_id);
    }
}

#[contractimpl]
impl ExecutionEntryPoint for MultisigContract {
    /// Execute a function on a target contract.
    ///
    /// This provides a secure mechanism for the smart account to invoke
    /// functions on other contracts, such as updating policy
    /// configurations. Requires smart account authorization.
    ///
    /// # Arguments
    ///
    /// * `target` - Address of the contract to invoke
    /// * `target_fn` - Function name to call on the target contract
    /// * `target_args` - Arguments to pass to the target function
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();

        e.invoke_contract::<Val>(&target, &target_fn, target_args);
    }
}

#[contractimpl]
impl Upgradeable for MultisigContract {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, _operator: Address) {
        e.current_contract_address().require_auth();
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}
