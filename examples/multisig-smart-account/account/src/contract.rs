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

#[contractimpl(contracttrait)]
impl SmartAccount for MultisigContract {}

#[contractimpl(contracttrait)]
impl ExecutionEntryPoint for MultisigContract {}

#[contractimpl]
impl Upgradeable for MultisigContract {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, _operator: Address) {
        e.current_contract_address().require_auth();
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}
