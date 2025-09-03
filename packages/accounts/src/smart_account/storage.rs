//! # Smart Account Storage - Context-Centric Authorization
//!
//! This module implements a flexible, context-centric authorization system for
//! smart accounts that separates concerns into three key dimensions:
//!
//! ## Architecture Overview
//!
//! ### **Who** - Signers
//! - **Native Signers**: Soroban native `Address` that use built-in signature
//!   verification
//! - **Delegated Signers**: Raw public key bytes paired with external verifier
//!   contracts for custom cryptographic verification (e.g., different signature
//!   schemes)
//!
//! ### **What** - Context Rules  
//! - Rules define authorization requirements for specific contexts (contract
//!   calls, deployments).
//! - Each rule must contain at least one signer or one policy and can have an
//!   optional expiration (`valid_until`) defined by a ledger sequence.
//! - Multiple rules can exist for the same context type with different signer
//!   sets and policies.
//! - Context types: `Default` (any context), `CallContract(Address)`,
//!   `CreateContract(BytesN<32>)`.
//! - Each rule specifies required signers and optional policies
//!
//! ### **How** - Policies
//! - External contracts that customize signer behavior and add business logic
//! - Policies can enforce additional constraints like basic or weighted
//!   threshold requirements
//! - All policies in a rule must be satisfied (all-or-nothing enforcement)
//!
//! ## Key Design Principles
//!
//! ### Context-Centric Approach
//! The system flips traditional key-centric reasoning to focus on **what you're
//! authorizing** rather than **which keys are signing**. This mirrors familiar
//! web2 OAuth patterns where users primarily care about the scope/permissions
//! being granted, not the underlying keys.
//!
//! ### Multiple Rules Per Context
//! Different authorization requirements for the same context:
//! - Admin config: 2-of-3 threshold for contract calls
//! - User config: 3-of-5 threshold for the same contract calls
//! - Emergency config: 1-of-1 with additional policy constraints
//!
//! ## Authorization Matching Algorithm
//!
//! When verifying a context with provided signers:
//!
//! I. Get all non-expired rules for the specific context type, plus default
//! rules.
//! II. For each rule (iteration starts from the last-stored, which prevails in
//! case of conflicting non-expired rules):
//!     1. Identify authenticated signers out of all stored signers.
//!     2.a. If there are policies, verify they can be enforced:
//!         - If all policies can be satisfied, return success.
//!         - Otherwise, move to the next rule.
//!     2.b. If there are no policies:
//!         - Return success only if all signers are authenticated.
//!         - Otherwise, move to the next rule.
//! III. If no context rule satisfies the above conditions, authorization fails.
//!
//! ## Benefits
//!
//! - **User-Friendly**: Focus on authorization scope rather than key management
//! - **Extensible**: Policies allow custom business logic without core changes
//! - **Flexible**: Multiple authorization paths for different user groups
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // Rule 1: Admin group - 3 of 3 signers, no policies
//! ContextRule {
//!     context_type: CallContract(token_contract),
//!     signers: [admin1, admin2, admin3],
//!     policies: [],
//! }
//!
//! // Rule 2: User group - 3 of 5 signers, with spending limit policy  
//! ContextRule {
//!     context_type: CallContract(token_contract),
//!     signers: [user1, user2, user3, user4, user5],
//!     policies: [threshold_policy, spending_limit_policy],
//! }
//! ```

use soroban_sdk::{
    auth::{
        Context, ContractContext, ContractExecutable, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    contracterror, contracttype,
    crypto::Hash,
    panic_with_error, Address, Bytes, BytesN, Env, IntoVal, Map, String, Val, Vec,
};

use crate::{policies::PolicyClient, verifiers::VerifierClient};

// TODO: proper enumeration
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SmartAccountError {
    ContextRuleNotFound = 0,
    ConflictingContextRule = 1,
    UnvalidatedContext = 2,
    DelegatedVerificationFailed = 3,
    NoSignersAndPolicies = 4,
    PastValidUntil = 5,
    SignerNotFound = 6,
    DuplicateSigner = 7,
    PolicyNotFound = 8,
    DuplicatePolicy = 9,
}

#[contracttype]
pub enum SmartAccountStorageKey {
    Signers(u32),         // maps context id to Vec<Signer>
    Policies(u32),        // maps context id to Vec<Address>
    Ids(ContextRuleType), // maps context type to ids per context type
    Meta(u32),            // maps context id to Meta
    NextId,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Signer {
    Native(Address),
    Delegated(Address, Bytes),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Signatures(pub Map<Signer, Bytes>);

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ContextRuleType {
    Default, // signers of default rules can auth any context
    CallContract(Address),
    CreateContract(BytesN<32>),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub name: String,
    pub context_type: ContextRuleType,
    pub valid_until: Option<u32>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContextRule {
    pub id: u32,
    pub context_type: ContextRuleType,
    pub name: String,
    pub signers: Vec<Signer>,
    pub policies: Vec<Address>,
    pub valid_until: Option<u32>,
}

// ################## QUERY STATE ##################

pub fn get_context_rule(e: &Env, id: u32) -> ContextRule {
    let meta: Meta = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Meta(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    let signers: Vec<Signer> =
        e.storage().persistent().get(&SmartAccountStorageKey::Signers(id)).unwrap_or(Vec::new(e));

    let policies: Vec<Address> =
        e.storage().persistent().get(&SmartAccountStorageKey::Policies(id)).unwrap_or(Vec::new(e));

    ContextRule {
        id,
        context_type: meta.context_type,
        name: meta.name,
        signers,
        policies,
        valid_until: meta.valid_until,
    }
}

pub fn get_context_rules(e: &Env, context_rule_type: &ContextRuleType) -> Vec<ContextRule> {
    let ids = e
        .storage()
        .persistent()
        .get::<_, Vec<u32>>(&SmartAccountStorageKey::Ids(context_rule_type.clone()))
        .unwrap_or(Vec::new(e));

    let mut rules = Vec::new(e);
    for id in ids.iter() {
        if e.storage().persistent().has(&SmartAccountStorageKey::Meta(id)) {
            rules.push_back(get_context_rule(e, id));
        }
    }
    rules
}

pub fn get_valid_context_rules(e: &Env, context_key: ContextRuleType) -> Vec<ContextRule> {
    let matched_ids = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Ids(context_key))
        .unwrap_or(Vec::new(e));

    let default_ids = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Ids(ContextRuleType::Default))
        .unwrap_or(Vec::new(e));

    let get_rules = |ids: Vec<u32>| -> Vec<ContextRule> {
        let mut rules = Vec::new(e);
        for id in ids.iter() {
            if e.storage().persistent().has(&SmartAccountStorageKey::Meta(id)) {
                let rule = get_context_rule(e, id);
                match rule.valid_until {
                    // skip if expired
                    Some(seq) if seq < e.ledger().sequence() => continue,
                    // push front so that we start from the last added when iterating
                    _ => rules.push_front(rule),
                }
            }
        }
        rules
    };

    let mut final_rules = get_rules(matched_ids);
    // append defaults so that there is always a fallback
    final_rules.append(&get_rules(default_ids));

    final_rules
}

pub fn get_authenticated_signers(
    e: &Env,
    rule_signers: &Vec<Signer>,
    all_signers: &Vec<Signer>,
) -> Vec<Signer> {
    let mut authenticated = Vec::new(e);
    for rule_signer in rule_signers.iter() {
        if all_signers.contains(&rule_signer) {
            authenticated.push_back(rule_signer);
        }
    }
    authenticated
}

pub fn get_validated_context(
    e: &Env,
    context: &Context,
    all_signers: &Vec<Signer>,
) -> (ContextRule, Context, Vec<Signer>) {
    let context_rules = match context.clone() {
        #[rustfmt::skip]
        Context::Contract(ContractContext { contract, .. }) => {
            get_valid_context_rules(e, ContextRuleType::CallContract(contract))
        },
        Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => get_valid_context_rules(e, ContextRuleType::CreateContract(wasm)),
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => get_valid_context_rules(e, ContextRuleType::CreateContract(wasm)),
    };

    for context_rule in context_rules.iter() {
        let ContextRule { signers: rule_signers, policies, .. } = context_rule.clone();

        let authenticated_signers = get_authenticated_signers(e, &rule_signers, all_signers);
        if policies.is_empty() {
            // if no policies, return only if all rule signers are authenticated
            if rule_signers.len() == authenticated_signers.len() {
                return (context_rule, context.clone(), authenticated_signers.clone());
            }
        } else {
            // otherwise, only if all policies can be enforced
            if can_enforce_all_policies(
                e,
                context,
                &policies,
                &rule_signers,
                &authenticated_signers,
            ) {
                return (context_rule, context.clone(), authenticated_signers.clone());
            }
        }
    }

    panic_with_error!(e, SmartAccountError::UnvalidatedContext)
}

pub fn authenticate(e: &Env, signature_payload: &Hash<32>, signatures: &Signatures) {
    for (signer, sig_data) in signatures.0.iter() {
        match signer {
            Signer::Delegated(verifier, key_data) => {
                let sig_payload = Bytes::from_array(e, &signature_payload.to_bytes().to_array());
                if !VerifierClient::new(e, &verifier).verify(&sig_payload, &key_data, &sig_data) {
                    panic_with_error!(e, SmartAccountError::DelegatedVerificationFailed)
                }
            }
            Signer::Native(addr) => {
                let args = (signature_payload.clone(),).into_val(e);
                addr.require_auth_for_args(args)
            }
        }
    }
}

pub fn can_enforce_all_policies(
    e: &Env,
    context: &Context,
    policies: &Vec<Address>,
    rule_signers: &Vec<Signer>,
    matched_signers: &Vec<Signer>,
) -> bool {
    for policy in policies.iter() {
        // policies are all or nothing
        if !PolicyClient::new(e, &policy).can_enforce(
            context,
            rule_signers,
            matched_signers,
            &e.current_contract_address(),
        ) {
            return false;
        }
    }
    true
}

pub fn do_check_auth(
    e: Env,
    signature_payload: Hash<32>,
    signatures: Signatures,
    auth_contexts: Vec<Context>,
) -> Result<(), SmartAccountError> {
    authenticate(&e, &signature_payload, &signatures);

    let mut validated_contexts = Vec::new(&e);
    for context in auth_contexts.iter() {
        validated_contexts.push_back(get_validated_context(&e, &context, &signatures.0.keys()));
    }

    // After collecting validated context rules and authenticated signers, call for
    // every policy `PolicyClient::enforce` to trigger the state-changing
    // effects if any.
    for (rule, context, authenticated_signers) in validated_contexts.iter() {
        let ContextRule { signers, policies, .. } = rule;
        for policy in policies.iter() {
            enforce_policy(&e, &policy, &context, &signers, &authenticated_signers);
        }
    }

    Ok(())
}

// ################## CHANGE STATE ##################

pub fn enforce_policy(
    e: &Env,
    policy: &Address,
    context: &Context,
    rule_signers: &Vec<Signer>,
    matched_signers: &Vec<Signer>,
) {
    PolicyClient::new(e, policy).enforce(
        context,
        rule_signers,
        matched_signers,
        &e.current_contract_address(),
    );
}

pub fn add_context_rule(
    e: &Env,
    context_type: &ContextRuleType,
    name: String,
    valid_until: Option<u32>,
    signers: Vec<Signer>,
    policies: Vec<Address>,
    policies_params: Vec<Val>,
) -> ContextRule {
    let mut id = e.storage().persistent().get(&SmartAccountStorageKey::NextId).unwrap_or(0u32);
    let mut same_key_ids: Vec<u32> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Ids(context_type.clone()))
        .unwrap_or(Vec::new(e));

    // Check for at least one of signers or policies > 0
    if signers.is_empty() && policies.is_empty() {
        panic_with_error!(e, SmartAccountError::NoSignersAndPolicies)
    }

    // Check for duplicate signers
    let mut unique_signers = Vec::new(e);
    for signer in signers.iter() {
        if unique_signers.contains(&signer) {
            panic_with_error!(e, SmartAccountError::DuplicateSigner)
        }
        unique_signers.push_back(signer);
    }

    // Check for duplicate policies
    let mut unique_policies = Vec::new(e);
    for policy in policies.iter() {
        if unique_policies.contains(&policy) {
            panic_with_error!(e, SmartAccountError::DuplicatePolicy)
        }
        unique_policies.push_back(policy);
    }

    // Check valid_until
    if let Some(valid_until) = valid_until {
        if valid_until < e.ledger().sequence() {
            panic_with_error!(e, SmartAccountError::PastValidUntil)
        }
    }

    // Store meta information
    let meta = Meta { name: name.clone(), context_type: context_type.clone(), valid_until };
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    // Store signers
    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);

    // Store policies and install them
    e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies);
    for (policy, param) in policies.iter().zip(policies_params.iter()) {
        PolicyClient::new(e, &policy).install(&param, &e.current_contract_address());
    }

    // Update ids list
    same_key_ids.push_back(id);
    e.storage().persistent().set(&SmartAccountStorageKey::Ids(context_type.clone()), &same_key_ids);

    // Increment next id
    id += 1;
    e.storage().persistent().set(&SmartAccountStorageKey::NextId, &id);

    ContextRule {
        id: id - 1,
        context_type: context_type.clone(),
        name,
        signers,
        policies,
        valid_until,
    }
}

pub fn update_context_rule_name(e: &Env, id: u32, name: String) -> ContextRule {
    let existing_rule = get_context_rule(e, id);

    // Update only the name in meta information
    let meta = Meta {
        name: name.clone(),
        context_type: existing_rule.context_type.clone(),
        valid_until: existing_rule.valid_until,
    };
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    ContextRule {
        id,
        context_type: existing_rule.context_type,
        name,
        signers: existing_rule.signers,
        policies: existing_rule.policies,
        valid_until: existing_rule.valid_until,
    }
}

pub fn update_context_rule_valid_until(e: &Env, id: u32, valid_until: Option<u32>) -> ContextRule {
    let existing_rule = get_context_rule(e, id);

    // check valid_until
    if let Some(valid_until) = valid_until {
        if valid_until < e.ledger().sequence() {
            panic_with_error!(e, SmartAccountError::PastValidUntil)
        }
    }

    // Update only the valid_until in meta information
    let meta = Meta {
        name: existing_rule.name.clone(),
        context_type: existing_rule.context_type.clone(),
        valid_until,
    };
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    ContextRule {
        id,
        context_type: existing_rule.context_type,
        name: existing_rule.name,
        signers: existing_rule.signers,
        policies: existing_rule.policies,
        valid_until,
    }
}

pub fn remove_context_rule(e: &Env, id: u32) {
    let context_rule = get_context_rule(e, id);

    // Uninstall all policies
    for policy in context_rule.policies.iter() {
        PolicyClient::new(e, &policy).uninstall(&e.current_contract_address());
    }

    // Remove all storage entries for this context rule
    e.storage().persistent().remove(&SmartAccountStorageKey::Meta(id));
    e.storage().persistent().remove(&SmartAccountStorageKey::Signers(id));
    e.storage().persistent().remove(&SmartAccountStorageKey::Policies(id));

    // Remove from ids list
    let ids_key = SmartAccountStorageKey::Ids(context_rule.context_type);
    let mut ids = e.storage().persistent().get::<_, Vec<u32>>(&ids_key).unwrap_or(Vec::new(e));

    if let Some(pos) = ids.iter().rposition(|i| i == id) {
        ids.remove(pos as u32);
        e.storage().persistent().set(&ids_key, &ids);
    }
}

// ################## SIGNER MANAGEMENT ##################

pub fn add_signer(e: &Env, id: u32, signer: Signer) {
    let mut signers: Vec<Signer> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Signers(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Check if signer already exists
    if signers.contains(&signer) {
        panic_with_error!(e, SmartAccountError::DuplicateSigner)
    }

    signers.push_back(signer);
    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);
}

pub fn remove_signer(e: &Env, id: u32, signer: Signer) {
    let mut signers: Vec<Signer> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Signers(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Find and remove the signer
    if let Some(pos) = signers.iter().rposition(|s| s == signer) {
        signers.remove(pos as u32);
        e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);
    } else {
        panic_with_error!(e, SmartAccountError::SignerNotFound)
    }
}

// ################## POLICY MANAGEMENT ##################

pub fn add_policy(e: &Env, id: u32, policy: Address, install_param: Val) {
    let mut policies: Vec<Address> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Policies(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Check if policy already exists
    if policies.contains(&policy) {
        panic_with_error!(e, SmartAccountError::DuplicatePolicy)
    }

    // Install the policy
    PolicyClient::new(e, &policy).install(&install_param, &e.current_contract_address());

    policies.push_back(policy);
    e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies);
}

pub fn remove_policy(e: &Env, id: u32, policy: Address) {
    let mut policies: Vec<Address> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::Policies(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Find and remove the policy
    if let Some(pos) = policies.iter().rposition(|p| p == policy) {
        // Uninstall the policy
        PolicyClient::new(e, &policy).uninstall(&e.current_contract_address());

        policies.remove(pos as u32);
        e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies);
    } else {
        panic_with_error!(e, SmartAccountError::PolicyNotFound)
    }
}
