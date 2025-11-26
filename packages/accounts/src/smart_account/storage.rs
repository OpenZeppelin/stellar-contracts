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

use cvlr::nondet::{self, Nondet};
use cvlr_soroban::{nondet_address, nondet_bytes, nondet_bytes_n, nondet_string, nondet_vec};
#[cfg(feature = "certora")]
use soroban_sdk::FromVal;
use soroban_sdk::{
    auth::{
        Context, ContractContext, ContractExecutable, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    contracttype,
    crypto::Hash,
    panic_with_error,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, IntoVal, Map, String, TryFromVal, Val, Vec,
};

#[cfg(not(feature = "certora"))]
use crate::smart_account::{
    emit_context_rule_added, emit_context_rule_removed, emit_context_rule_updated,
    emit_policy_added, emit_policy_removed, emit_signer_added, emit_signer_removed,
};
#[cfg(feature = "certora")]
use crate::{
    policies::Policy,
    smart_account::specs::policy::{Params, PolicyContract},
};
use crate::{
    policies::{self, PolicyClient},
    smart_account::{
        SmartAccountError, MAX_CONTEXT_RULES, MAX_POLICIES, MAX_SIGNERS,
        SMART_ACCOUNT_EXTEND_AMOUNT, SMART_ACCOUNT_TTL_THRESHOLD,
    },
    verifiers::VerifierClient,
};

/// Storage keys for smart account data.
#[contracttype]
pub enum SmartAccountStorageKey {
    /// Storage key for signers of a context rule.
    /// Maps context rule ID to `Vec<Signer>`.
    Signers(u32),
    /// Storage key for policies of a context rule.
    /// Maps context rule ID to `Vec<Address>`.
    Policies(u32),
    /// Storage key for context rule IDs by type.
    /// Maps `ContextRuleType` to `Vec<u32>` of rule IDs.
    Ids(ContextRuleType),
    /// Storage key for context rule metadata.
    /// Maps context rule ID to `Meta`.
    Meta(u32),
    /// Storage key for the next available context rule ID.
    NextId,
    /// Storage key defining the fingerprint each context rule.
    Fingerprint(BytesN<32>),
    /// Storage key for the count of active context rules.
    /// Used to enforce MAX_CONTEXT_RULES limit.
    Count,
}

/// Represents different types of signers in the smart account system.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signer {
    /// A delegated signer that uses built-in signature verification.
    Delegated(Address),
    /// An external signer with custom verification logic.
    /// Contains the verifier contract address and the public key data.
    External(Address, Bytes),
}

impl Nondet for Signer {
    fn nondet() -> Self {
        if bool::nondet() {
            Signer::Delegated(nondet_address())
        } else {
            Signer::External(nondet_address(), nondet_bytes())
        }
    }
}

/// A collection of signatures mapped to their respective signers.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Signatures(pub Map<Signer, Bytes>);

/// Types of contexts that can be authorized by smart account rules.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextRuleType {
    /// Default rules that can authorize any context.
    Default,
    /// Rules specific to calling a particular contract.
    CallContract(Address),
    /// Rules specific to creating a contract with a particular WASM hash.
    CreateContract(BytesN<32>),
}

impl Nondet for ContextRuleType {
    fn nondet() -> Self {
        match u8::nondet() % 3 {
            0 => ContextRuleType::Default,
            1 => ContextRuleType::CallContract(nondet_address()),
            2 => ContextRuleType::CreateContract(nondet_bytes_n()),
            _ => panic!("unreachable"),
        }
    }
}

/// Metadata for a context rule.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// Human-readable name for the context rule.
    pub name: String,
    /// The type of context this rule applies to.
    pub context_type: ContextRuleType,
    /// Optional expiration ledger sequence for the rule.
    pub valid_until: Option<u32>,
}

impl Nondet for Meta {
    fn nondet() -> Self {
        Meta {
            name: nondet_string(),
            context_type: ContextRuleType::nondet(),
            valid_until: Option::nondet(),
        }
    }
}

/// A complete context rule defining authorization requirements.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContextRule {
    /// Unique identifier for the context rule.
    pub id: u32,
    /// The type of context this rule applies to.
    pub context_type: ContextRuleType,
    /// Human-readable name for the context rule.
    pub name: String,
    /// List of signers authorized by this rule.
    pub signers: Vec<Signer>,
    /// List of policy contracts that must be satisfied.
    pub policies: Vec<Address>,
    /// Optional expiration ledger sequence for the rule.
    pub valid_until: Option<u32>,
}

impl Nondet for ContextRule {
    fn nondet() -> Self {
        ContextRule {
            id: u32::nondet(),
            context_type: ContextRuleType::nondet(),
            name: nondet_string(),
            signers: nondet_vec(),
            policies: nondet_vec(),
            valid_until: Option::<u32>::nondet(),
        }
    }
}

// ################## QUERY STATE ##################

/// Retrieves a complete context rule by its ID.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The unique identifier of the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
pub fn get_context_rule(e: &Env, id: u32) -> ContextRule {
    let meta_key = SmartAccountStorageKey::Meta(id);
    let meta: Meta = get_persistent_entry(e, &meta_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    let signers_key = SmartAccountStorageKey::Signers(id);
    let signers: Vec<Signer> = get_persistent_entry(e, &signers_key).unwrap_or_else(|| Vec::new(e));

    let policies_key = SmartAccountStorageKey::Policies(id);
    let policies: Vec<Address> =
        get_persistent_entry(e, &policies_key).unwrap_or_else(|| Vec::new(e));

    ContextRule {
        id,
        context_type: meta.context_type,
        name: meta.name,
        signers,
        policies,
        valid_until: meta.valid_until,
    }
}

/// Retrieves all context rules of a specific context type. Returns a vector of
/// all context rules matching the specified type, including expired rules.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_type` - The type of context rules to retrieve.
pub fn get_context_rules(e: &Env, context_rule_type: &ContextRuleType) -> Vec<ContextRule> {
    let ids_key = SmartAccountStorageKey::Ids(context_rule_type.clone());
    let ids: Vec<u32> = get_persistent_entry(e, &ids_key).unwrap_or_else(|| Vec::new(e));

    #[cfg(not(feature = "certora"))]
    {
        Vec::from_iter(e, ids.iter().map(|id| get_context_rule(e, id)))
    }

    #[cfg(feature = "certora")]
    {
        let mut v: Vec<ContextRule> = Vec::new(e);
        for id in ids.iter() {
            v.push_back(get_context_rule(e, id));
        }
        return v;
    }
}

/// Retrieves all valid (non-expired) context rules for a specific context type,
/// including default rules as fallback. Returns rules ordered with most
/// recently added first for proper authorization precedence.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_key` - The context type to find valid rules for.
pub fn get_valid_context_rules(e: &Env, context_key: &ContextRuleType) -> Vec<ContextRule> {
    let matched_ids_key = SmartAccountStorageKey::Ids(context_key.clone());
    let matched_ids: Vec<u32> =
        get_persistent_entry(e, &matched_ids_key).unwrap_or_else(|| Vec::new(e));

    let default_ids_key = SmartAccountStorageKey::Ids(ContextRuleType::Default);
    let default_ids: Vec<u32> =
        get_persistent_entry(e, &default_ids_key).unwrap_or_else(|| Vec::new(e));

    let get_rules = |ids: Vec<u32>| -> Vec<ContextRule> {
        let mut rules = Vec::new(e);
        for id in ids.iter() {
            let rule = get_context_rule(e, id);
            match rule.valid_until {
                // skip if expired
                Some(seq) if seq < e.ledger().sequence() => continue,
                // push front so that we start from the last added when iterating
                _ => rules.push_front(rule),
            }
        }
        rules
    };

    let mut final_rules = get_rules(matched_ids);
    // append defaults so that there is always a fallback
    final_rules.append(&get_rules(default_ids));

    final_rules
}

/// Filters rule signers to find which ones are present in the provided signer
/// list. Returns a vector of signers that exist in both the rule and the
/// provided signer list.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `rule_signers` - The signers required by a context rule.
/// * `all_signers` - The signers provided for authentication.
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

/// Validates a context against all applicable rules and returns the matching
/// rule with authenticated signers. Returns a tuple of the matched context
/// rule, the validated context, and the authenticated signers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context to validate.
/// * `all_signers` - The signers provided for authentication.
///
/// # Errors
///
/// * [`SmartAccountError::UnvalidatedContext`] - When no context rule can
///   validate the provided context and signers.
pub fn get_validated_context(
    e: &Env,
    context: &Context,
    all_signers: &Vec<Signer>,
) -> (ContextRule, Context, Vec<Signer>) {
    let context_rules = match context.clone() {
        #[rustfmt::skip]
        Context::Contract(ContractContext { contract, .. }) => {
            get_valid_context_rules(e, &ContextRuleType::CallContract(contract))
        },
        Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => get_valid_context_rules(e, &ContextRuleType::CreateContract(wasm)),
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => get_valid_context_rules(e, &ContextRuleType::CreateContract(wasm)),
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
            if can_enforce_all_policies(e, context, &context_rule, &authenticated_signers) {
                return (context_rule, context.clone(), authenticated_signers.clone());
            }
        }
    }

    panic_with_error!(e, SmartAccountError::UnvalidatedContext)
}

/// Authenticates all provided signatures against their respective signers.
/// Verifies both `Address` authorizations and delegated signatures through
/// external verifier contracts.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signature_payload` - The hash of the data that was signed.
/// * `signatures` - The signatures mapped to their signers.
///
/// # Errors
///
/// * [`SmartAccountError::ExternalVerificationFailed`] - When an external
///   signature fails verification through its verifier contract.
pub fn authenticate(e: &Env, signature_payload: &Hash<32>, signers: &Map<Signer, Bytes>) {
    for (signer, sig_data) in signers.iter() {
        match signer {
            Signer::External(verifier, key_data) => {
                let sig_payload = Bytes::from_array(e, &signature_payload.to_bytes().to_array());
                if !VerifierClient::new(e, &verifier).verify(
                    &sig_payload,
                    &key_data.into_val(e),
                    &sig_data.into_val(e),
                ) {
                    panic_with_error!(e, SmartAccountError::ExternalVerificationFailed)
                }
            }
            Signer::Delegated(addr) => {
                let args = (signature_payload.clone(),).into_val(e);
                addr.require_auth_for_args(args)
            }
        }
    }
}

/// Checks if all policies in a rule can be enforced with the provided signers.
/// Returns `true` only if all policies can be satisfied, `false` otherwise.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `context_rule` - The context rule.
/// * `matched_signers` - The authenticated signers.
pub fn can_enforce_all_policies(
    e: &Env,
    context: &Context,
    context_rule: &ContextRule,
    matched_signers: &Vec<Signer>,
) -> bool {
    for policy in context_rule.policies.iter() {
        // policies are all or nothing
        #[cfg(not(feature = "certora"))]
        if !PolicyClient::new(e, &policy).can_enforce(
            context,
            matched_signers,
            context_rule,
            &e.current_contract_address(),
        ) {
            return false;
        }
        #[cfg(feature = "certora")]
        if PolicyContract::can_enforce(
            e,
            context.clone(),
            matched_signers.clone(),
            context_rule.clone(),
            e.current_contract_address(),
        ) {
            return false;
        }
    }
    true
}

/// Validates signers and policies against maximum limits and minimum
/// requirements.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The vector of signers to validate.
/// * `policies` - The vector of policies to validate.
///
/// # Errors
///
/// * [`SmartAccountError::TooManySigners`] - When there are more than
///   MAX_SIGNERS signers.
/// * [`SmartAccountError::TooManyPolicies`] - When there are more than
///   MAX_POLICIES policies.
/// * [`SmartAccountError::NoSignersAndPolicies`] - When there are no signers
///   and no policies.
pub fn validate_signers_and_policies(e: &Env, signers: &Vec<Signer>, policies: &Vec<Address>) {
    // Check maximum limits
    if signers.len() > MAX_SIGNERS {
        panic_with_error!(e, SmartAccountError::TooManySigners);
    }

    if policies.len() > MAX_POLICIES {
        panic_with_error!(e, SmartAccountError::TooManyPolicies);
    }

    // Check minimum requirements - must have at least one signer or one policy
    if signers.is_empty() && policies.is_empty() {
        panic_with_error!(e, SmartAccountError::NoSignersAndPolicies);
    }
}

/// Performs complete authorization check for multiple contexts. Authenticates
/// signatures, validates contexts against rules, and enforces all applicable
/// policies. Returns success if all contexts are successfully authorized.
///
/// This function is meant to be used in `__check_auth` of a smart account.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `signature_payload` - The hash of the data that was signed.
/// * `signatures` - The signatures mapped to their signers.
/// * `auth_contexts` - The contexts to authorize.
///
/// # Errors
///
/// * [`SmartAccountError::ExternalVerificationFailed`] - When signature
///   verification fails.
/// * [`SmartAccountError::UnvalidatedContext`] - When a context cannot be
///   validated against any rule.
pub fn do_check_auth(
    e: &Env,
    signature_payload: &Hash<32>,
    signatures: &Signatures,
    auth_contexts: &Vec<Context>,
) -> Result<(), SmartAccountError> {
    authenticate(e, signature_payload, &signatures.0);

    #[cfg(not(feature = "certora"))]
    let validated_contexts = Vec::from_iter(
        e,
        auth_contexts
            .iter()
            .map(|context| get_validated_context(e, &context, &signatures.0.keys())),
    );

    #[cfg(feature = "certora")]
    let validated_contexts = {
        let mut tmp = Vec::new(e);
        for context in auth_contexts {
            tmp.push_back(get_validated_context(e, &context, &signatures.0.keys()));
        }
        tmp
    };

    // After collecting validated context rules and authenticated signers, call for
    // every policy `PolicyClient::enforce` to trigger the state-changing
    // effects if any.
    for (rule, context, authenticated_signers) in validated_contexts.iter() {
        let ContextRule { policies, .. } = rule.clone();
        for policy in policies.iter() {
            #[cfg(not(feature = "certora"))]
            PolicyClient::new(e, &policy).enforce(
                &context,
                &authenticated_signers,
                &rule,
                &e.current_contract_address(),
            );
            #[cfg(feature = "certora")]
            PolicyContract::enforce(
                e,
                context.clone(),
                authenticated_signers.clone(),
                rule.clone(),
                e.current_contract_address(),
            );
        }
    }

    Ok(())
}

/// Computes a unique fingerprint for a context rule based on its signers,
/// policies, and expiration. The fingerprint is used to prevent duplicate rules
/// with identical authorization requirements.
///
/// The fingerprint is calculated by:
/// 1. Sorting signers and policies to ensure deterministic ordering
/// 2. Serializing them to XDR format
/// 3. Hashing the combined data with SHA-256
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The signers for the context rule.
/// * `policies` - The policies for the context rule.
/// * `valid_until` - Optional expiration ledger sequence.
///
/// # Errors
///
/// * [`SmartAccountError::DuplicateSigner`] - When duplicate signers are found
///   during sorting.
/// * [`SmartAccountError::DuplicatePolicy`] - When duplicate policies are found
///   during sorting.
pub fn compute_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signers: &Vec<Signer>,
    policies: &Vec<Address>,
) -> BytesN<32> {
    // let mut sorted_signers = Vec::new(e);
    // for signer in signers.iter() {
    //     match sorted_signers.binary_search(&signer) {
    //         Ok(_) => panic_with_error!(e, SmartAccountError::DuplicateSigner),
    //         Err(pos) => sorted_signers.insert(pos, signer),
    //     }
    // }

    // let mut sorted_policies = Vec::new(e);
    // for policy in policies.iter() {
    //     match sorted_policies.binary_search(&policy) {
    //         Ok(_) => panic_with_error!(e, SmartAccountError::DuplicatePolicy),
    //         Err(pos) => sorted_policies.insert(pos, policy),
    //     }
    // }

    // #[cfg(not(feature = "certora"))]
    // let mut rule_data = context_type.to_xdr(e);
    // #[cfg(feature = "certora")]
    // let mut rule_data = context_type.clone().to_xdr(e);
    // rule_data.append(&sorted_signers.to_xdr(e));
    // rule_data.append(&sorted_policies.to_xdr(e));

    // e.crypto().sha256(&rule_data).to_bytes()
    nondet_bytes_n()
}

// ################## CHANGE STATE ##################

/// Creates a new context rule with the specified configuration. Returns the
/// created context rule with a unique ID. Installs all specified policies
/// during creation.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_type` - The type of context this rule applies to.
/// * `name` - Human-readable name for the context rule.
/// * `valid_until` - Optional expiration ledger sequence.
/// * `signers` - List of signers authorized by this rule.
/// * `policies` - Map of policy addresses to their installation parameters.
///
/// # Errors
///
/// * [`SmartAccountError::TooManyContextRules`] - When the number of context
///   rules exceeds MAX_CONTEXT_RULES (15).
/// * [`SmartAccountError::NoSignersAndPolicies`] - When both signers and
///   policies are empty.
/// * [`SmartAccountError::TooManySigners`] - When the number of signers exceeds
///   MAX_SIGNERS (15).
/// * [`SmartAccountError::TooManyPolicies`] - When the number of policies
///   exceeds MAX_POLICIES (5).
/// * [`SmartAccountError::DuplicateSigner`] - When the same signer appears
///   multiple times.
/// * [`SmartAccountError::PastValidUntil`] - When valid_until is in the past.
///
/// # Events
///
/// * topics - `["context_rule_added", id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>, signers: Vec<Signer>, policies: Vec<Address>]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn add_context_rule(
    e: &Env,
    context_type: &ContextRuleType,
    name: &String,
    valid_until: Option<u32>,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
) -> ContextRule {
    let id = e.storage().instance().get(&SmartAccountStorageKey::NextId).unwrap_or(0u32);

    let count = e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32);
    if count >= MAX_CONTEXT_RULES {
        panic_with_error!(e, SmartAccountError::TooManyContextRules);
    }

    let ids_key = SmartAccountStorageKey::Ids(context_type.clone());
    // Don't extend TTL here since we set this key later in the same function
    let mut same_key_ids: Vec<u32> =
        e.storage().persistent().get(&ids_key).unwrap_or_else(|| Vec::new(e));

    // Check for duplicate signers
    let mut unique_signers = Vec::new(e);
    for signer in signers.iter() {
        if unique_signers.contains(&signer) {
            panic_with_error!(e, SmartAccountError::DuplicateSigner);
        }
        unique_signers.push_back(signer);
    }

    // Check valid_until
    if let Some(valid_until) = valid_until {
        if valid_until < e.ledger().sequence() {
            panic_with_error!(e, SmartAccountError::PastValidUntil)
        }
    }

    #[cfg(not(feature = "certora"))]
    let policies_vec = Vec::from_iter(e, policies.keys());
    #[cfg(feature = "certora")]
    let policies_vec = {
        let mut tmp = Vec::new(e);
        for key in policies.keys() {
            tmp.push_back(key);
        }
        tmp
    };

    validate_signers_and_policies(e, &unique_signers, &policies_vec);
    validate_and_set_fingerprint(e, context_type, &unique_signers, &policies_vec);

    // Store meta information
    let meta = Meta { name: name.clone(), context_type: context_type.clone(), valid_until };
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    // Store signers
    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), signers);

    // Store policies
    e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies_vec);

    // Update ids list
    same_key_ids.push_back(id);
    e.storage().persistent().set(&SmartAccountStorageKey::Ids(context_type.clone()), &same_key_ids);

    let context_rule = ContextRule {
        id,
        context_type: context_type.clone(),
        name: name.clone(),
        signers: unique_signers,
        policies: policies_vec,
        valid_until,
    };

    // Install the policies
    for (policy, param) in policies.iter() {
        #[cfg(not(feature = "certora"))]
        PolicyClient::new(e, &policy).install(&param, &context_rule, &e.current_contract_address());
        #[cfg(feature = "certora")]
        PolicyContract::install(
            &e,
            Params::from_val(e, &param),
            context_rule.clone(),
            e.current_contract_address(),
        );
    }

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_context_rule_added(e, &context_rule);

    // Increment next id
    e.storage().instance().set(&SmartAccountStorageKey::NextId, &(id + 1));

    // Increment count
    e.storage().instance().set(&SmartAccountStorageKey::Count, &(count + 1));

    context_rule
}

/// Updates the name of an existing context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule to update.
/// * `name` - The new name for the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
///
/// # Events
///
/// * topics - `["context_rule_updated", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn update_context_rule_name(e: &Env, id: u32, name: &String) -> ContextRule {
    let existing_rule = get_context_rule(e, id);

    // Update only the name in meta information
    let meta = Meta {
        name: name.clone(),
        context_type: existing_rule.context_type.clone(),
        valid_until: existing_rule.valid_until,
    };
    e.storage().persistent().set(&SmartAccountStorageKey::Meta(id), &meta);

    let context_rule = ContextRule {
        id,
        context_type: existing_rule.context_type,
        name: name.clone(),
        signers: existing_rule.signers,
        policies: existing_rule.policies,
        valid_until: existing_rule.valid_until,
    };

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_context_rule_updated(e, id, &meta);

    context_rule
}

/// Updates the expiration time for an existing context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule to update.
/// * `valid_until` - The new expiration ledger sequence for the rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::PastValidUntil`] - When valid_until is in the past.
///
/// # Events
///
/// * topics - `["context_rule_updated", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn update_context_rule_valid_until(e: &Env, id: u32, valid_until: Option<u32>) -> ContextRule {
    let existing_rule = get_context_rule(e, id);

    // Check valid_until
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

    let context_rule = ContextRule {
        id,
        context_type: existing_rule.context_type,
        name: existing_rule.name,
        signers: existing_rule.signers,
        policies: existing_rule.policies,
        valid_until,
    };

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_context_rule_updated(e, id, &meta);

    context_rule
}

/// Removes a context rule and uninstalls all its policies. Cleans up all
/// associated storage entries.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule to remove.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
///
/// # Events
///
/// * topics - `["context_rule_removed", context_rule_id: u32]`
/// * data - `[]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn remove_context_rule(e: &Env, id: u32) {
    let context_rule = get_context_rule(e, id);

    // Uninstall all policies
    for policy in context_rule.policies.iter() {
        #[cfg(not(feature = "certora"))]
        PolicyClient::new(e, &policy).uninstall(&context_rule, &e.current_contract_address());
        #[cfg(feature = "certora")]
        PolicyContract::uninstall(e, context_rule.clone(), e.current_contract_address());
    }

    // Remove all storage entries for this context rule
    e.storage().persistent().remove(&SmartAccountStorageKey::Meta(id));
    e.storage().persistent().remove(&SmartAccountStorageKey::Signers(id));
    e.storage().persistent().remove(&SmartAccountStorageKey::Policies(id));
    remove_fingerprint(
        e,
        &context_rule.context_type,
        &context_rule.signers,
        &context_rule.policies,
    );

    // Remove from ids list
    let ids_key = SmartAccountStorageKey::Ids(context_rule.context_type);
    // Don't extend TTL here since we set this key later in the same function
    let mut ids =
        e.storage().persistent().get::<_, Vec<u32>>(&ids_key).unwrap_or_else(|| Vec::new(e));

    if let Some(pos) = ids.iter().rposition(|i| i == id) {
        ids.remove(pos as u32);
        e.storage().persistent().set(&ids_key, &ids);
    }

    // Decrement count
    let count: u32 = e.storage().instance().get(&SmartAccountStorageKey::Count).expect("to be set");
    // if count is set, it can be safely assumed it's greater than 0
    e.storage().instance().set(&SmartAccountStorageKey::Count, &(count - 1));

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_context_rule_removed(e, id);
}

// ################## SIGNER MANAGEMENT ##################

/// Adds a new signer to an existing context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule.
/// * `signer` - The signer to add to the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::DuplicateSigner`] - When the signer already exists in
///   the context rule.
/// * [`SmartAccountError::TooManySigners`] - When adding this signer would
///   exceed MAX_SIGNERS (15).
///
/// # Events
///
/// * topics - `["signer_added", context_rule_id: u32]`
/// * data - `[signer: Signer]`
///
/// # Security Warning
///
/// * **Threshold Policy Consideration:** If the ContextRule contains a
///   threshold-based policy (e.g., simple_threshold), adding signers may
///   silently weaken the security guarantee. For example, a strict N-of-N
///   multisig becomes an N-of-(N+M) multisig after adding M signers. **Always
///   update the policy threshold AFTER adding signers** to maintain the desired
///   security level, especially for N-of-N multisig configurations.
///
/// * This function modifies storage without requiring authorization. Ensure
///   proper access control is implemented at the contract level.
pub fn add_signer(e: &Env, id: u32, signer: &Signer) {
    let rule = get_context_rule(e, id);
    let mut signers = rule.signers.clone();

    // Check if signer already exists
    if signers.contains(signer) {
        panic_with_error!(e, SmartAccountError::DuplicateSigner)
    }

    signers.push_back(signer.clone());

    // Validate the updated signers and policies
    validate_signers_and_policies(e, &signers, &rule.policies);

    validate_and_set_fingerprint(e, &rule.context_type, &signers, &rule.policies);
    // Remove the old fingerprint
    remove_fingerprint(e, &rule.context_type, &rule.signers, &rule.policies);

    e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_signer_added(e, id, signer);
}

/// Removes a signer from an existing context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule.
/// * `signer` - The signer to remove from the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::SignerNotFound`] - When the specified signer is not
///   found in the context rule.
///
/// # Events
///
/// * topics - `["signer_removed", context_rule_id: u32]`
/// * data - `[signer: Signer]`
///
/// # Security Warning
///
/// * **Threshold Policy Consideration:** If the ContextRule contains a
///   threshold-based policy (e.g., simple_threshold), removing signers may
///   cause a denial of service if the remaining signers fall below the policy's
///   threshold. **Always update the policy threshold BEFORE removing signers**
///   to ensure the threshold remains achievable with the remaining signer set.
///
/// * This function modifies storage without requiring authorization. Ensure
///   proper access control is implemented at the contract level.
pub fn remove_signer(e: &Env, id: u32, signer: &Signer) {
    let rule = get_context_rule(e, id);
    let mut signers = rule.signers.clone();

    // Find and remove the signer
    if let Some(pos) = signers.iter().rposition(|s| s == *signer) {
        signers.remove(pos as u32);

        // Validate that the rule still has at least one signer or one policy
        validate_signers_and_policies(e, &signers, &rule.policies);

        validate_and_set_fingerprint(e, &rule.context_type, &signers, &rule.policies);
        // Remove the old fingerprint
        remove_fingerprint(e, &rule.context_type, &rule.signers, &rule.policies);

        e.storage().persistent().set(&SmartAccountStorageKey::Signers(id), &signers);

        // Emit event
        #[cfg(not(feature = "certora"))]
        emit_signer_removed(e, id, signer);
    } else {
        panic_with_error!(e, SmartAccountError::SignerNotFound)
    }
}

// ################## POLICY MANAGEMENT ##################

/// Adds a new policy to an existing context rule and installs it.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule.
/// * `policy` - The address of the policy contract to add.
/// * `install_param` - The installation parameter for the policy.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::DuplicatePolicy`] - When the policy already exists in
///   the context rule.
/// * [`SmartAccountError::TooManyPolicies`] - When adding this policy would
///   exceed MAX_POLICIES (5).
///
/// # Events
///
/// * topics - `["policy_added", context_rule_id: u32]`
/// * data - `[policy: Address, install_param: Val]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn add_policy(e: &Env, id: u32, policy: &Address, install_param: Val) {
    let rule = get_context_rule(e, id);
    let mut policies = rule.policies.clone();

    // Check if policy already exists
    if policies.contains(policy) {
        panic_with_error!(e, SmartAccountError::DuplicatePolicy)
    }

    // Install the policy
    #[cfg(not(feature = "certora"))]
    PolicyClient::new(e, policy).install(&install_param, &rule, &e.current_contract_address());
    #[cfg(feature = "certora")]
    PolicyContract::install(
        e,
        Params::from_val(e, &install_param),
        rule.clone(),
        e.current_contract_address(),
    );

    policies.push_back(policy.clone());

    // Validate the updated signers and policies
    validate_signers_and_policies(e, &rule.signers, &policies);

    validate_and_set_fingerprint(e, &rule.context_type, &rule.signers, &policies);
    // Remove the old fingerprint
    remove_fingerprint(e, &rule.context_type, &rule.signers, &rule.policies);

    e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies);

    // Emit event
    #[cfg(not(feature = "certora"))]
    emit_policy_added(e, id, policy, install_param);
}

/// Removes a policy from an existing context rule and uninstalls it.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule.
/// * `policy` - The address of the policy contract to remove.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::PolicyNotFound`] - When the specified policy is not
///   found in the context rule.
///
/// # Events
///
/// * topics - `["policy_removed", context_rule_id: u32]`
/// * data - `[policy: Address]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn remove_policy(e: &Env, id: u32, policy: &Address) {
    let rule = get_context_rule(e, id);
    let mut policies = rule.policies.clone();

    // Find and remove the policy
    if let Some(pos) = policies.iter().rposition(|p| p == *policy) {
        policies.remove(pos as u32);

        // Validate that the rule still has at least one signer or one policy
        validate_signers_and_policies(e, &rule.signers, &policies);

        validate_and_set_fingerprint(e, &rule.context_type, &rule.signers, &policies);
        // Remove the old fingerprint
        // remove_fingerprint(e, &rule.context_type, &rule.signers, &rule.policies);

        // Uninstall the policy
        #[cfg(not(feature = "certora"))]
        PolicyClient::new(e, policy).uninstall(&rule, &e.current_contract_address());
        #[cfg(feature = "certora")]
        PolicyContract::uninstall(e, rule, e.current_contract_address());

        e.storage().persistent().set(&SmartAccountStorageKey::Policies(id), &policies);

        // Emit event
        #[cfg(not(feature = "certora"))]
        emit_policy_removed(e, id, policy);
    } else {
        panic_with_error!(e, SmartAccountError::PolicyNotFound)
    }
}

/// Validates that a context rule with the given authorization requirements
/// doesn't already exist, then stores its fingerprint. This prevents creating
/// duplicate rules with identical signers, policies, and expiration.
///
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The signers for the context rule.
/// * `policies` - The policies for the context rule.
/// * `valid_until` - Optional expiration ledger sequence.
///
/// # Errors
///
/// * [`SmartAccountError::DuplicateContextRule`] - When a rule with identical
///   authorization requirements already exists.
pub fn validate_and_set_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signers: &Vec<Signer>,
    policies: &Vec<Address>,
) {
    let fingerprint = compute_fingerprint(e, context_type, signers, policies);
    let fingerprint_key = SmartAccountStorageKey::Fingerprint(fingerprint);

    if e.storage().persistent().has(&fingerprint_key) {
        panic_with_error!(e, SmartAccountError::DuplicateContextRule)
    } else {
        e.storage().persistent().set(&fingerprint_key, &true);
    }
}

// ################## HELPERS ##################

/// Removes a context rule's fingerprint from storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_type` - The type of context this rule applies to.
/// * `signers` - The signers for the context rule.
/// * `policies` - The policies for the context rule.
fn remove_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signers: &Vec<Signer>,
    policies: &Vec<Address>,
) {
    let fingerprint = compute_fingerprint(e, context_type, signers, policies);
    e.storage().persistent().remove(&SmartAccountStorageKey::Fingerprint(fingerprint));
}

/// Helper function that tries to retrieve a persistent storage value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `key` - The storage key to retrieve the value for.
pub(crate) fn get_persistent_entry<T: TryFromVal<Env, Val>>(
    e: &Env,
    key: &SmartAccountStorageKey,
) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        e.storage().persistent().extend_ttl(
            key,
            SMART_ACCOUNT_TTL_THRESHOLD,
            SMART_ACCOUNT_EXTEND_AMOUNT,
        );
    })
}
