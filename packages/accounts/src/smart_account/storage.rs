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
//! III. If no rule satisfies the above conditions, authorization fails.
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
    panic_with_error, Address, Bytes, BytesN, Env, IntoVal, Map, String, Vec,
};

use crate::{policies::PolicyClient, verifiers::VerifierClient};

// TODO: proper enumeration
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SmartAccountError {
    ContextRuleNotFound = 0,
    ConflictingContextRule = 1,
    UnverifiedContext = 2,
    DelegatedVerificationFailed = 3,
}

#[contracttype]
pub enum SmartAccountStorageKey {
    ContextRule(u32),                // -> ContextRule
    ContextRuleIds(ContextRuleType), // -> ids per context type
    ContextRuleNextId,
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
pub struct ContextRuleVal {
    pub name: String,
    pub signers: Vec<Signer>,
    pub policies: Vec<Address>,
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

pub fn get_context_rule(e: &Env, id: u32) -> ContextRule {
    e.storage()
        .persistent()
        .get(&SmartAccountStorageKey::ContextRule(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound))
}

pub fn get_context_rules(e: &Env, context_rule_type: &ContextRuleType) -> Vec<ContextRule> {
    let ids = e
        .storage()
        .persistent()
        .get::<_, Vec<u32>>(&SmartAccountStorageKey::ContextRuleIds(context_rule_type.clone()))
        .unwrap_or(Vec::new(e));

    let mut rules = Vec::new(e);
    for id in ids.iter() {
        if let Some(rule) = e.storage().persistent().get(&SmartAccountStorageKey::ContextRule(id)) {
            rules.push_back(rule);
        }
    }
    rules
}

pub fn authenticate(e: &Env, signature_payload: &Hash<32>, signatures: &Signatures) {
    for (signer, sig) in signatures.0.iter() {
        match signer {
            Signer::Delegated(verifier, pub_key) => {
                let hash = Bytes::from_array(e, &signature_payload.to_bytes().to_array());
                let mut sig_data = Bytes::new(e);
                sig_data.append(&pub_key);
                sig_data.append(&sig);
                if !VerifierClient::new(e, &verifier).verify(&hash, &sig_data) {
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

    panic_with_error!(e, SmartAccountError::UnverifiedContext)
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
            &e.current_contract_address(),
            context,
            rule_signers,
            matched_signers,
        ) {
            return false;
        }
    }
    true
}

pub fn enforce_policy(
    e: &Env,
    policy: &Address,
    context: &Context,
    rule_signers: &Vec<Signer>,
    matched_signers: &Vec<Signer>,
) {
    PolicyClient::new(e, policy).enforce(
        &e.current_contract_address(),
        context,
        rule_signers,
        matched_signers,
    );
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

fn get_valid_context_rules(e: &Env, context_key: ContextRuleType) -> Vec<ContextRule> {
    let matched_ids = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::ContextRuleIds(context_key))
        .unwrap_or(Vec::new(e));

    let default_ids = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::ContextRuleIds(ContextRuleType::Default))
        .unwrap_or(Vec::new(e));

    let get_rules = |ids: Vec<u32>| -> Vec<ContextRule> {
        let mut rules = Vec::new(e);
        for id in ids.iter() {
            if let Some(rule) = e
                .storage()
                .persistent()
                .get::<_, ContextRule>(&SmartAccountStorageKey::ContextRule(id))
            {
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

pub fn add_context_rule(
    e: &Env,
    context_rule_type: &ContextRuleType,
    context_rule_val: &ContextRuleVal,
) {
    let mut id =
        e.storage().persistent().get(&SmartAccountStorageKey::ContextRuleNextId).unwrap_or(0u32);
    let mut same_key_ids: Vec<u32> = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::ContextRuleIds(context_rule_type.clone()))
        .unwrap_or(Vec::new(e));

    let ContextRuleVal { signers, policies, name, valid_until } = context_rule_val.clone();
    // check signers.len or policies.len > 0
    // check valid_until
    let rule = ContextRule {
        id,
        context_type: context_rule_type.clone(),
        name,
        signers,
        policies,
        valid_until,
    };

    e.storage().persistent().set(&SmartAccountStorageKey::ContextRule(id), &rule);

    same_key_ids.push_back(id);
    e.storage()
        .persistent()
        .set(&SmartAccountStorageKey::ContextRuleIds(context_rule_type.clone()), &same_key_ids);

    id += 1;
    e.storage().persistent().set(&SmartAccountStorageKey::ContextRuleNextId, &id);
}
