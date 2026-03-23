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

use crate::{
    policies::PolicyClient,
    smart_account::{
        emit_context_rule_added, emit_context_rule_meta_updated, emit_context_rule_removed,
        emit_policy_added, emit_policy_deregistered, emit_policy_registered, emit_policy_removed,
        emit_signer_added, emit_signer_deregistered, emit_signer_registered, emit_signer_removed,
        SmartAccountError, MAX_EXTERNAL_KEY_SIZE, MAX_NAME_SIZE, MAX_POLICIES, MAX_SIGNERS,
        SMART_ACCOUNT_EXTEND_AMOUNT, SMART_ACCOUNT_TTL_THRESHOLD,
    },
    verifiers::VerifierClient,
};

/// Storage keys for smart account data.
#[contracttype]
pub enum SmartAccountStorageKey {
    /// Storage key for combined context rule data.
    /// Maps context rule ID to `ContextRuleEntry` (signer IDs, policies, and
    /// metadata stored in a single entry).
    ContextRuleData(u32),
    /// Storage key for the next available context rule ID.
    NextId,
    /// Storage key defining the fingerprint each context rule.
    Fingerprint(BytesN<32>),
    /// Storage key for the count of active context rules.
    Count,
    /// Storage key for global signer data.
    /// Maps signer ID to `SignerEntry` (stored once, referenced by rules).
    SignerData(u32),
    /// Storage key for signer lookup by hash.
    /// Maps `sha256(Signer XDR)` to signer ID for deduplication.
    SignerLookup(BytesN<32>),
    /// Storage key for the next available global signer ID (monotonically
    /// increasing).
    NextSignerId,
    /// Storage key for global policy data.
    /// Maps policy ID to `PolicyEntry`.
    PolicyData(u32),
    /// Storage key for policy lookup by address.
    /// Maps policy `Address` to its policy ID for deduplication.
    PolicyLookup(Address),
    /// Storage key for the next available global policy ID (monotonically
    /// increasing).
    NextPolicyId,
}

/// Combines context rule metadata, signer IDs, and policy addresses into a
/// single storage entry, reducing persistent reads per auth check from 3 to 1.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContextRuleEntry {
    /// Human-readable name for the context rule.
    pub name: String,
    /// The type of context this rule applies to.
    pub context_type: ContextRuleType,
    /// Optional expiration ledger sequence.
    pub valid_until: Option<u32>,
    /// Global signer IDs referenced by this rule.
    pub signer_ids: Vec<u32>,
    /// Policy IDs referenced by this rule.
    pub policy_ids: Vec<u32>,
}

/// Combines signer data and its reference count into a single storage entry.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SignerEntry {
    /// The signer stored in the global registry.
    pub signer: Signer,
    /// Number of context rules referencing this signer.
    pub count: u32,
}

/// Combines policy data and its reference count into a single storage entry.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PolicyEntry {
    /// The policy address stored in the global registry.
    pub policy: Address,
    /// Number of context rules referencing this policy.
    pub count: u32,
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

/// The authorization payload passed to `__check_auth`, bundling cryptographic
/// proofs with context rule selection.
///
/// This struct carries two distinct pieces of information that are both
/// required for authorization but cannot be derived from each other:
///
/// - `signers` maps each [`Signer`] to its raw signature bytes, providing
///   cryptographic proof that the signer actually signed the transaction
///   payload. A context rule stores which signer *identities* are authorized
///   (via `signer_ids`), but the rule does not contain the signatures
///   themselves — those must be supplied here.
///
/// - `context_rule_ids` tells the system which rule to validate for each auth
///   context. Because multiple rules can exist for the same context type, the
///   caller must explicitly select one per context rather than relying on
///   auto-discovery. Each entry is aligned by index with the `auth_contexts`
///   passed to `__check_auth`.
///
/// The length of `context_rule_ids` must equal the number of auth contexts;
/// a mismatch is rejected with
/// [`SmartAccountError::ContextRuleIdsLengthMismatch`].
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AuthPayload {
    /// Signature data mapped to each signer.
    pub signers: Map<Signer, Bytes>,
    /// Per-context rule IDs, aligned by index with `auth_contexts`.
    pub context_rule_ids: Vec<u32>,
}

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
    let entry: ContextRuleEntry =
        get_persistent_entry(e, &SmartAccountStorageKey::ContextRuleData(id))
            .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    ContextRule {
        id,
        context_type: entry.context_type,
        name: entry.name,
        signers: get_signers(e, &entry.signer_ids),
        policies: get_policies(e, &entry.policy_ids),
        valid_until: entry.valid_until,
    }
}

/// Retrieves the number of all context rules, including expired ones. Defaults
/// to 0.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn get_context_rules_count(e: &Env) -> u32 {
    e.storage().instance().get(&SmartAccountStorageKey::Count).unwrap_or(0u32)
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

/// Validates a context against a specific rule identified by `id`. Checks
/// expiration, context type compatibility, and signer requirements.
///
/// For rules without policies, all signers must be authenticated. For rules
/// with policies, validation is deferred to `enforce()` — the policy is the
/// authority on what signers are needed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context to validate.
/// * `all_signers` - The signers provided for authentication.
/// * `id` - The context rule ID to validate against.
///
/// # Errors
///
/// * [`SmartAccountError::UnvalidatedContext`] - When the rule is expired, its
///   context type does not match, or (for rules without policies) not all
///   signers are authenticated.
/// * refer to [`get_context_rule`] errors.
pub fn get_validated_context_by_id(
    e: &Env,
    context: &Context,
    all_signers: &Vec<Signer>,
    id: u32,
) -> (ContextRule, Context, Vec<Signer>) {
    let context_rule = get_context_rule(e, id);

    // Reject expired rules.
    if let Some(valid_until) = context_rule.valid_until {
        if valid_until < e.ledger().sequence() {
            panic_with_error!(e, SmartAccountError::UnvalidatedContext);
        }
    }

    // The rule's context type must match the actual context, or be Default
    // (which applies to any context).
    let required_type = match context.clone() {
        Context::Contract(ContractContext { contract, .. }) => {
            ContextRuleType::CallContract(contract)
        }
        Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => ContextRuleType::CreateContract(wasm),
        Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            executable: ContractExecutable::Wasm(wasm),
            ..
        }) => ContextRuleType::CreateContract(wasm),
    };

    let context_type_matches = context_rule.context_type == ContextRuleType::Default
        || context_rule.context_type == required_type;

    if !context_type_matches {
        panic_with_error!(e, SmartAccountError::UnvalidatedContext);
    }

    let ContextRule { signers: ref rule_signers, ref policies, .. } = context_rule;
    let authenticated_signers = get_authenticated_signers(e, rule_signers, all_signers);

    if policies.is_empty() {
        // Without policies, all rule signers must be authenticated.
        if rule_signers.len() != authenticated_signers.len() {
            panic_with_error!(e, SmartAccountError::UnvalidatedContext);
        }
    }
    // With policies, defer full validation to enforce().

    (context_rule, context.clone(), authenticated_signers)
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

/// Validates signer IDs and policy IDs against maximum limits and minimum
/// requirements.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_ids` - The vector of signer IDs to validate.
/// * `policy_ids` - The vector of policy IDs to validate.
///
/// # Errors
///
/// * [`SmartAccountError::TooManySigners`] - When there are more than
///   MAX_SIGNERS signers.
/// * [`SmartAccountError::TooManyPolicies`] - When there are more than
///   MAX_POLICIES policies.
/// * [`SmartAccountError::NoSignersAndPolicies`] - When there are no signers
///   and no policies.
pub fn validate_signers_and_policies(e: &Env, signer_ids: &Vec<u32>, policy_ids: &Vec<u32>) {
    // Check maximum limits
    if signer_ids.len() > MAX_SIGNERS {
        panic_with_error!(e, SmartAccountError::TooManySigners);
    }

    if policy_ids.len() > MAX_POLICIES {
        panic_with_error!(e, SmartAccountError::TooManyPolicies);
    }

    // Check minimum requirements - must have at least one signer or one policy
    if signer_ids.is_empty() && policy_ids.is_empty() {
        panic_with_error!(e, SmartAccountError::NoSignersAndPolicies);
    }
}

/// Validates that a signer's external key data does not exceed the maximum
/// allowed size.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer` - The signer to validate.
///
/// # Errors
///
/// * [`SmartAccountError::KeyDataTooLarge`] - When the external signer key data
///   exceeds [`MAX_EXTERNAL_KEY_SIZE`] bytes.
pub fn validate_signer_key_size(e: &Env, signer: &Signer) {
    if let Signer::External(_, key_data) = signer {
        if key_data.len() > MAX_EXTERNAL_KEY_SIZE {
            panic_with_error!(e, SmartAccountError::KeyDataTooLarge);
        }
    }
}

/// Validates that a context rule name does not exceed the maximum allowed
/// length.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The context rule name to validate.
///
/// # Errors
///
/// * [`SmartAccountError::NameTooLong`] - When the name exceeds
///   [`MAX_NAME_SIZE`] bytes.
pub fn validate_context_rule_name(e: &Env, name: &String) {
    if name.len() > MAX_NAME_SIZE {
        panic_with_error!(e, SmartAccountError::NameTooLong);
    }
}

/// Performs complete authorization check for multiple contexts. Authenticates
/// signatures, validates contexts against rules, and enforces all applicable
/// policies. Returns success if all contexts are successfully authorized.
///
/// This function is meant to be used in `__check_auth` of a smart account.
///
/// Each entry in [`AuthPayload::context_rule_ids`] specifies the rule ID to
/// validate against for the corresponding auth context (by index). Its length
/// must equal `auth_contexts.len()`.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `signature_payload` - The hash of the data that was signed.
/// * `signatures` - The signatures and per-context rule IDs.
/// * `auth_contexts` - The contexts to authorize.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleIdsLengthMismatch`] - When
///   `context_rule_ids` has a different length than `auth_contexts`.
/// * refer to [`authenticate`] errors.
/// * refer to [`get_validated_context_by_id`] errors.
pub fn do_check_auth(
    e: &Env,
    signature_payload: &Hash<32>,
    signatures: &AuthPayload,
    auth_contexts: &Vec<Context>,
) -> Result<(), SmartAccountError> {
    if signatures.context_rule_ids.len() != auth_contexts.len() {
        panic_with_error!(e, SmartAccountError::ContextRuleIdsLengthMismatch);
    }

    authenticate(e, signature_payload, &signatures.signers);

    // Validate all contexts against their specified rules.
    let validated_contexts = Vec::from_iter(
        e,
        auth_contexts.iter().enumerate().map(|(i, context)| {
            let all_signers = signatures.signers.keys();
            let context_rule_id = signatures.context_rule_ids.get_unchecked(i as u32);

            get_validated_context_by_id(e, &context, &all_signers, context_rule_id)
        }),
    );

    // Enforce all policies.
    for (rule, context, authenticated_signers) in validated_contexts.iter() {
        for policy in rule.policies.iter() {
            PolicyClient::new(e, &policy).enforce(
                &context,
                &authenticated_signers,
                &rule,
                &e.current_contract_address(),
            );
        }
    }

    Ok(())
}

/// Computes a unique fingerprint for a context rule based on its type, signer
/// IDs, and policy IDs. The fingerprint is used to prevent duplicate rules with
/// identical authorization requirements.
///
/// The fingerprint is computed by:
/// 1. Sorting signer IDs and policy IDs to ensure consistent ordering
/// 2. Serializing the context type, sorted signer IDs, and sorted policy IDs to
///    XDR
/// 3. Hashing the combined data with SHA-256
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_type` - The type of context this rule applies to.
/// * `signer_ids` - The signer IDs for the context rule.
/// * `policy_ids` - The policy IDs for the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::DuplicateSigner`] - When duplicate signer IDs are
///   found during sorting.
/// * [`SmartAccountError::DuplicatePolicy`] - When duplicate policy IDs are
///   found during sorting.
pub fn compute_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signer_ids: &Vec<u32>,
    policy_ids: &Vec<u32>,
) -> BytesN<32> {
    let mut sorted_signer_ids = Vec::new(e);
    for id in signer_ids.iter() {
        match sorted_signer_ids.binary_search(id) {
            Ok(_) => panic_with_error!(e, SmartAccountError::DuplicateSigner),
            Err(pos) => sorted_signer_ids.insert(pos, id),
        }
    }

    let mut sorted_policy_ids = Vec::new(e);
    for id in policy_ids.iter() {
        match sorted_policy_ids.binary_search(id) {
            Ok(_) => panic_with_error!(e, SmartAccountError::DuplicatePolicy),
            Err(pos) => sorted_policy_ids.insert(pos, id),
        }
    }

    let mut rule_data = context_type.to_xdr(e);
    rule_data.append(&sorted_signer_ids.to_xdr(e));
    rule_data.append(&sorted_policy_ids.to_xdr(e));

    e.crypto().sha256(&rule_data).to_bytes()
}

/// Checks if any signer in `signers` has the same canonical key identity as
/// `new_signer`.
///
/// For [`Signer::External`] signers, this calls the verifier's
/// `batch_canonicalize_key` to compare cryptographic identities rather than raw
/// bytes. Two external signers with the same verifier are considered
/// duplicates if their canonical key representations match, even if their
/// raw key bytes differ.
///
/// For [`Signer::Delegated`] signers, this falls back to direct byte
/// equality since `Address` values are already canonical.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The existing list of signers to check against.
/// * `new_signer` - The signer to check for duplicates.
pub fn contains_canonical_duplicate(e: &Env, signers: &Vec<Signer>, new_signer: &Signer) -> bool {
    match new_signer {
        Signer::External(verifier, key_data) => {
            let client = VerifierClient::new(e, verifier);

            let mut key_batch = Vec::new(e);

            // Filter signers with the same verifier
            for existing in signers.iter() {
                if let Signer::External(existing_verifier, existing_key_data) = existing {
                    if existing_verifier == *verifier {
                        key_batch.push_back(existing_key_data.into_val(e));
                    }
                }
            }

            if key_batch.is_empty() {
                return false;
            }

            key_batch.push_back(key_data.into_val(e));

            let canonical_batch = client.batch_canonicalize_key(&key_batch);
            let new_canonical = canonical_batch.last().expect("new canonical key to be present");

            canonical_batch.iter().rev().skip(1).any(|canonical| canonical == new_canonical)
        }
        Signer::Delegated(_) => signers.contains(new_signer),
    }
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
/// * [`SmartAccountError::DuplicateSigner`] - When the same signer appears
///   multiple times.
/// * [`SmartAccountError::PastValidUntil`] - When `valid_until` is in the past.
/// * [`SmartAccountError::MathOverflow`] - When the context rule ID counter has
///   reached `u32::MAX` and cannot be incremented.
/// * refer to [`validate_context_rule_name`] errors.
/// * refer to [`validate_signer_key_size`] errors.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// * topics - `["context_rule_added", id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>, signer_ids: Vec<u32>, policy_ids: Vec<u32>]`
///
/// For each signer not previously registered in the global registry:
/// * topics - `["signer_registered", signer_id: u32]`
/// * data - `[signer: Signer]`
///
/// For each policy not previously registered in the global registry:
/// * topics - `["policy_registered", policy_id: u32]`
/// * data - `[policy: Address]`
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
    validate_context_rule_name(e, name);

    let id = e.storage().instance().get(&SmartAccountStorageKey::NextId).unwrap_or(0u32);

    let count = get_context_rules_count(e);

    // Check for duplicate signers using canonical key comparison
    let mut unique_signers = Vec::new(e);
    for signer in signers.iter() {
        validate_signer_key_size(e, &signer);
        if contains_canonical_duplicate(e, &unique_signers, &signer) {
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

    let policies_vec = Vec::from_iter(e, policies.keys());

    // Register signers in global registry and collect their IDs
    let signer_ids: Vec<u32> =
        Vec::from_iter(e, unique_signers.iter().map(|s| register_signer(e, &s)));

    // Register policies in global registry and collect their IDs
    let policy_ids: Vec<u32> =
        Vec::from_iter(e, policies_vec.iter().map(|p| register_policy(e, &p)));

    validate_signers_and_policies(e, &signer_ids, &policy_ids);

    set_fingerprint(e, context_type, &signer_ids, &policy_ids);

    // Store all context rule data in a single entry
    e.storage().persistent().set(
        &SmartAccountStorageKey::ContextRuleData(id),
        &ContextRuleEntry {
            name: name.clone(),
            context_type: context_type.clone(),
            valid_until,
            signer_ids: signer_ids.clone(),
            policy_ids: policy_ids.clone(),
        },
    );

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
        PolicyClient::new(e, &policy).install(&param, &context_rule, &e.current_contract_address());
    }

    emit_context_rule_added(e, id, name, context_type, valid_until, &signer_ids, &policy_ids);

    // Increment next id
    let next_id =
        id.checked_add(1).unwrap_or_else(|| panic_with_error!(e, SmartAccountError::MathOverflow));
    e.storage().instance().set(&SmartAccountStorageKey::NextId, &next_id);

    // Increment count, overflow will be caught from next_id, next_id is always >=
    // count
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
/// * refer to [`validate_context_rule_name`] errors.
///
/// # Events
///
/// * topics - `["context_rule_meta_updated", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn update_context_rule_name(e: &Env, id: u32, name: &String) -> ContextRule {
    validate_context_rule_name(e, name);

    let data_key = SmartAccountStorageKey::ContextRuleData(id);
    let mut entry: ContextRuleEntry = e
        .storage()
        .persistent()
        .get(&data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    entry.name = name.clone();
    e.storage().persistent().set(&data_key, &entry);

    emit_context_rule_meta_updated(e, id, name, &entry.valid_until);

    ContextRule {
        id,
        context_type: entry.context_type,
        name: name.clone(),
        valid_until: entry.valid_until,
        signers: get_signers(e, &entry.signer_ids),
        policies: get_policies(e, &entry.policy_ids),
    }
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
/// * topics - `["context_rule_meta_updated", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn update_context_rule_valid_until(e: &Env, id: u32, valid_until: Option<u32>) -> ContextRule {
    if let Some(valid_until) = valid_until {
        if valid_until < e.ledger().sequence() {
            panic_with_error!(e, SmartAccountError::PastValidUntil)
        }
    }

    let data_key = SmartAccountStorageKey::ContextRuleData(id);
    let mut entry: ContextRuleEntry = e
        .storage()
        .persistent()
        .get(&data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    entry.valid_until = valid_until;
    e.storage().persistent().set(&data_key, &entry);

    emit_context_rule_meta_updated(e, id, &entry.name, &valid_until);

    ContextRule {
        id,
        context_type: entry.context_type,
        name: entry.name,
        valid_until,
        signers: get_signers(e, &entry.signer_ids),
        policies: get_policies(e, &entry.policy_ids),
    }
}

/// Removes a context rule and tries to uninstall all its policies. Cleans up
/// all associated storage entries.
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
/// If this was the last context rule referencing a signer:
/// * topics - `["signer_deregistered", signer_id: u32]`
/// * data - `[]`
///
/// If this was the last context rule referencing a policy:
/// * topics - `["policy_deregistered", policy_id: u32]`
/// * data - `[]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn remove_context_rule(e: &Env, id: u32) {
    let entry: ContextRuleEntry = e
        .storage()
        .persistent()
        .get(&SmartAccountStorageKey::ContextRuleData(id))
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    let policies = get_policies(e, &entry.policy_ids);
    let context_rule = ContextRule {
        id,
        context_type: entry.context_type.clone(),
        name: entry.name,
        signers: get_signers(e, &entry.signer_ids),
        policies: policies.clone(),
        valid_until: entry.valid_until,
    };

    for (policy, policy_id) in policies.iter().zip(&entry.policy_ids) {
        // `try_uninstall` so that if the policy panics, context rule removal can be
        // completed. This prevents a malicious or misconfigured policy from blocking a
        // context rule removal.
        let _ = PolicyClient::new(e, &policy)
            .try_uninstall(&context_rule, &e.current_contract_address());

        // Deregister policies from global registry
        deregister_policy(e, policy_id);
    }

    for signer_id in entry.signer_ids.iter() {
        deregister_signer(e, signer_id);
    }

    remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    e.storage().persistent().remove(&SmartAccountStorageKey::ContextRuleData(id));

    // Decrement count
    let count: u32 = e.storage().instance().get(&SmartAccountStorageKey::Count).expect("to be set");
    // if count is set, it can be safely assumed it's greater than 0
    e.storage().instance().set(&SmartAccountStorageKey::Count, &(count - 1));

    emit_context_rule_removed(e, id);
}

// ################## SIGNER MANAGEMENT ##################

/// Adds a new signer to an existing context rule, returning the assigned
/// signer ID.
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
/// * refer to [`validate_signer_key_size`] errors.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// * topics - `["signer_added", context_rule_id: u32]`
/// * data - `[signer_id: u32]`
///
/// If the signer is not previously registered in the global registry:
/// * topics - `["signer_registered", signer_id: u32]`
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
pub fn add_signer(e: &Env, id: u32, signer: &Signer) -> u32 {
    validate_signer_key_size(e, signer);

    // Get current entry to access existing IDs
    let data_key = SmartAccountStorageKey::ContextRuleData(id);
    let mut entry: ContextRuleEntry = get_persistent_entry(e, &data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Check if signer already exists using canonical key comparison (against
    // resolved signers)
    let signers = get_signers(e, &entry.signer_ids);
    if contains_canonical_duplicate(e, &signers, signer) {
        panic_with_error!(e, SmartAccountError::DuplicateSigner)
    }

    remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    let new_signer_id = register_signer(e, signer);

    entry.signer_ids.push_back(new_signer_id);

    validate_signers_and_policies(e, &entry.signer_ids, &entry.policy_ids);

    set_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    e.storage().persistent().set(&data_key, &entry);

    emit_signer_added(e, id, new_signer_id);

    new_signer_id
}

/// Removes a signer from an existing context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `signer_id` - The signer ID to remove from the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::SignerNotFound`] - When the specified signer is not
///   found in the context rule.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// * topics - `["signer_removed", context_rule_id: u32]`
/// * data - `[signer_id: u32]`
///
/// If this was the last context rule referencing the signer:
/// * topics - `["signer_deregistered", signer_id: u32]`
/// * data - `[]`
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
pub fn remove_signer(e: &Env, context_rule_id: u32, signer_id: u32) {
    let data_key = SmartAccountStorageKey::ContextRuleData(context_rule_id);
    let mut entry: ContextRuleEntry = get_persistent_entry(e, &data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    if let Some(pos) = entry.signer_ids.iter().rposition(|s| s == signer_id) {
        remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

        entry.signer_ids.remove(pos as u32);

        validate_signers_and_policies(e, &entry.signer_ids, &entry.policy_ids);

        set_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

        e.storage().persistent().set(&data_key, &entry);
        deregister_signer(e, signer_id);

        emit_signer_removed(e, context_rule_id, signer_id);
    } else {
        panic_with_error!(e, SmartAccountError::SignerNotFound)
    }
}

/// Adds multiple signers to an existing context rule in a single operation.
///
/// More efficient than calling [`add_signer`] in a loop because it resolves
/// existing signers once, removes and resets the fingerprint once, and writes
/// the [`ContextRuleEntry`] once regardless of how many signers are added.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `id` - The ID of the context rule.
/// * `signers` - The signers to add to the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::DuplicateSigner`] - When any signer already exists in
///   the context rule or appears more than once in `signers`.
/// * refer to [`validate_signer_key_size`] errors.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// For each signer added:
/// * topics - `["signer_added", context_rule_id: u32]`
/// * data - `[signer_id: u32]`
///
/// For each signer not previously registered in the global registry:
/// * topics - `["signer_registered", signer_id: u32]`
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
pub fn batch_add_signer(e: &Env, id: u32, signers: &Vec<Signer>) {
    let data_key = SmartAccountStorageKey::ContextRuleData(id);
    let mut entry: ContextRuleEntry = get_persistent_entry(e, &data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Resolve existing signers once for all duplicate checks.
    let mut existing_signers = get_signers(e, &entry.signer_ids);

    remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    for signer in signers.iter() {
        validate_signer_key_size(e, &signer);

        if contains_canonical_duplicate(e, &existing_signers, &signer) {
            panic_with_error!(e, SmartAccountError::DuplicateSigner);
        }

        let new_signer_id = register_signer(e, &signer);

        entry.signer_ids.push_back(new_signer_id);
        existing_signers.push_back(signer);

        emit_signer_added(e, id, new_signer_id);
    }

    validate_signers_and_policies(e, &entry.signer_ids, &entry.policy_ids);

    set_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    e.storage().persistent().set(&data_key, &entry);
}

// ################## POLICY MANAGEMENT ##################

/// Adds a new policy to an existing context rule, installs it, and returns
/// the assigned policy ID.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy` - The address of the policy contract to add.
/// * `install_param` - The installation parameter for the policy.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::DuplicatePolicy`] - When the policy already exists in
///   the context rule.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// * topics - `["policy_added", context_rule_id: u32]`
/// * data - `[policy_id: u32]`
///
/// If the policy is not previously registered in the global registry:
/// * topics - `["policy_registered", policy_id: u32]`
/// * data - `[policy: Address]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn add_policy(e: &Env, context_rule_id: u32, policy: &Address, install_param: Val) -> u32 {
    let data_key = SmartAccountStorageKey::ContextRuleData(context_rule_id);
    let mut entry: ContextRuleEntry = get_persistent_entry(e, &data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Register in global policy registry and get its ID
    let policy_id = register_policy(e, policy);

    // Check if policy already exists
    if entry.policy_ids.contains(policy_id) {
        panic_with_error!(e, SmartAccountError::DuplicatePolicy)
    }

    remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    entry.policy_ids.push_back(policy_id);

    validate_signers_and_policies(e, &entry.signer_ids, &entry.policy_ids);

    set_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

    e.storage().persistent().set(&data_key, &entry);

    let rule = ContextRule {
        id: context_rule_id,
        context_type: entry.context_type,
        name: entry.name,
        signers: get_signers(e, &entry.signer_ids),
        policies: get_policies(e, &entry.policy_ids),
        valid_until: entry.valid_until,
    };
    PolicyClient::new(e, policy).install(&install_param, &rule, &e.current_contract_address());

    emit_policy_added(e, context_rule_id, policy_id);

    policy_id
}

/// Removes a policy from an existing context rule and tries to uninstall it.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy_id` - The policy ID to remove from the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::ContextRuleNotFound`] - When the context rule with
///   the specified ID does not exist.
/// * [`SmartAccountError::PolicyNotFound`] - When the specified policy is not
///   found in the context rule.
/// * refer to [`validate_signers_and_policies`] errors.
///
/// # Events
///
/// * topics - `["policy_removed", context_rule_id: u32]`
/// * data - `[policy_id: u32]`
///
/// If this was the last context rule referencing the policy:
/// * topics - `["policy_deregistered", policy_id: u32]`
/// * data - `[]`
///
/// # Security Warning
///
/// This function modifies storage without requiring authorization. Ensure
/// proper access control is implemented at the contract level.
pub fn remove_policy(e: &Env, context_rule_id: u32, policy_id: u32) {
    let data_key = SmartAccountStorageKey::ContextRuleData(context_rule_id);
    let mut entry: ContextRuleEntry = get_persistent_entry(e, &data_key)
        .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::ContextRuleNotFound));

    // Find the policy position in the ID list
    if let Some(pos) = entry.policy_ids.iter().rposition(|p| p == policy_id) {
        let policies = get_policies(e, &entry.policy_ids);

        let rule = ContextRule {
            id: context_rule_id,
            context_type: entry.context_type.clone(),
            name: entry.name.clone(),
            signers: get_signers(e, &entry.signer_ids),
            policies: policies.clone(),
            valid_until: entry.valid_until,
        };
        let policy = policies.get_unchecked(pos as u32);
        let _ = PolicyClient::new(e, &policy).try_uninstall(&rule, &e.current_contract_address());

        remove_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

        entry.policy_ids.remove(pos as u32);

        validate_signers_and_policies(e, &entry.signer_ids, &entry.policy_ids);

        set_fingerprint(e, &entry.context_type, &entry.signer_ids, &entry.policy_ids);

        e.storage().persistent().set(&data_key, &entry);
        deregister_policy(e, policy_id);

        emit_policy_removed(e, context_rule_id, policy_id);
    } else {
        panic_with_error!(e, SmartAccountError::PolicyNotFound)
    }
}

// ################## HELPERS ##################

/// Registers a signer in the global registry, returning its unique ID.
///
/// If the signer already exists (by XDR hash), increments its reference count
/// and returns the existing ID. Otherwise, assigns a new monotonically
/// increasing ID, stores the signer data, and sets the reference count to 1.
/// IDs are never reused after deregistration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer` - The signer to register.
///
/// # Events
///
/// * topics - `["signer_registered", signer_id: u32]`
/// * data - `[signer: Signer]`
///
/// # Errors
///
/// [`SmartAccountError::MathOverflow`] - When `NextSignerId` has
/// reached `u32::MAX` and no new signer ID can be assigned.
fn register_signer(e: &Env, signer: &Signer) -> u32 {
    let hash = e.crypto().sha256(&signer.to_xdr(e)).to_bytes();
    let lookup_key = SmartAccountStorageKey::SignerLookup(hash.clone());

    if let Some(existing_id) = get_persistent_entry::<u32>(e, &lookup_key) {
        let data_key = SmartAccountStorageKey::SignerData(existing_id);
        let mut entry: SignerEntry =
            e.storage().persistent().get(&data_key).expect("signer entry to exist");

        entry.count += 1;
        e.storage().persistent().set(&data_key, &entry);

        existing_id
    } else {
        // Assign new signer next ID and store.
        let id: u32 =
            e.storage().instance().get(&SmartAccountStorageKey::NextSignerId).unwrap_or(0);
        e.storage().persistent().set(
            &SmartAccountStorageKey::SignerData(id),
            &SignerEntry { signer: signer.clone(), count: 1 },
        );
        e.storage().persistent().set(&lookup_key, &id);

        let next_signer_id = id
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::MathOverflow));
        e.storage().instance().set(&SmartAccountStorageKey::NextSignerId, &next_signer_id);

        emit_signer_registered(e, id, signer);

        id
    }
}

/// Decrements the reference count for a signer. If the count reaches zero,
/// removes all associated storage entries. The signer ID is never reused.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_id` - The signer ID to deregister.
///
/// # Events
///
/// * topics - `["signer_deregistered", signer_id: u32]`
/// * data - `[]`
fn deregister_signer(e: &Env, signer_id: u32) {
    let data_key = SmartAccountStorageKey::SignerData(signer_id);
    let entry: SignerEntry =
        e.storage().persistent().get(&data_key).expect("signer entry to exist");

    if entry.count <= 1 {
        // Last reference
        let hash = e.crypto().sha256(&entry.signer.to_xdr(e)).to_bytes();

        e.storage().persistent().remove(&data_key);
        e.storage().persistent().remove(&SmartAccountStorageKey::SignerLookup(hash));

        emit_signer_deregistered(e, signer_id);
    } else {
        e.storage()
            .persistent()
            .set(&data_key, &SignerEntry { signer: entry.signer, count: entry.count - 1 });
    }
}

/// Registers a policy in the global registry, returning its unique ID.
///
/// If the policy address already exists, increments its reference count and
/// returns the existing ID. Otherwise, assigns a new monotonically increasing
/// ID, stores the policy address, and sets the reference count to 1.
/// IDs are never reused after deregistration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `policy` - The policy address.
///
/// # Events
///
/// * topics - `["policy_registered", policy_id: u32]`
/// * data - `[policy: Address]`
///
/// # Errors
///
/// [`SmartAccountError::MathOverflow`] - When `NextPolicyId` has
/// reached `u32::MAX` and no new policy ID can be assigned.
fn register_policy(e: &Env, policy: &Address) -> u32 {
    let lookup_key = SmartAccountStorageKey::PolicyLookup(policy.clone());

    if let Some(existing_id) = get_persistent_entry::<u32>(e, &lookup_key) {
        let data_key = SmartAccountStorageKey::PolicyData(existing_id);
        let mut entry: PolicyEntry =
            e.storage().persistent().get(&data_key).expect("policy entry to exist");

        entry.count += 1;
        e.storage().persistent().set(&data_key, &entry);

        existing_id
    } else {
        // Assign new policy next ID and store.
        let id: u32 =
            e.storage().instance().get(&SmartAccountStorageKey::NextPolicyId).unwrap_or(0);
        e.storage().persistent().set(
            &SmartAccountStorageKey::PolicyData(id),
            &PolicyEntry { policy: policy.clone(), count: 1 },
        );
        e.storage().persistent().set(&lookup_key, &id);

        let next_policy_id = id
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, SmartAccountError::MathOverflow));
        e.storage().instance().set(&SmartAccountStorageKey::NextPolicyId, &next_policy_id);

        emit_policy_registered(e, id, policy);

        id
    }
}

/// Decrements the reference count for a policy. If the count reaches zero,
/// removes all associated storage entries. The policy ID is never reused.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `policy_id` - The policy ID to deregister.
///
/// # Events
///
/// * topics - `["policy_deregistered", policy_id: u32]`
/// * data - `[]`
fn deregister_policy(e: &Env, policy_id: u32) {
    let data_key = SmartAccountStorageKey::PolicyData(policy_id);
    let entry: PolicyEntry =
        e.storage().persistent().get(&data_key).expect("policy entry to exist");

    if entry.count <= 1 {
        // Last reference
        e.storage().persistent().remove(&data_key);
        e.storage().persistent().remove(&SmartAccountStorageKey::PolicyLookup(entry.policy));

        emit_policy_deregistered(e, policy_id);
    } else {
        e.storage()
            .persistent()
            .set(&data_key, &PolicyEntry { policy: entry.policy, count: entry.count - 1 });
    }
}

/// Validates that a context rule with the given authorization requirements
/// doesn't already exist, then stores its fingerprint. This prevents creating
/// duplicate rules with identical signers, policies, and context types.
///
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_type` - The type of context this rule applies to.
/// * `signers` - The signers for the context rule.
/// * `policies` - The policies for the context rule.
///
/// # Errors
///
/// * [`SmartAccountError::DuplicateContextRule`] - When a rule with identical
///   authorization requirements already exists.
pub(crate) fn set_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signer_ids: &Vec<u32>,
    policy_ids: &Vec<u32>,
) {
    let fingerprint = compute_fingerprint(e, context_type, signer_ids, policy_ids);
    let fingerprint_key = SmartAccountStorageKey::Fingerprint(fingerprint);

    if e.storage().persistent().has(&fingerprint_key) {
        panic_with_error!(e, SmartAccountError::DuplicateContextRule)
    } else {
        e.storage().persistent().set(&fingerprint_key, &true);
    }
}

/// Removes a context rule's fingerprint from storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_type` - The type of context this rule applies to.
/// * `signer_ids` - The signer IDs for the context rule.
/// * `policy_ids` - The policy IDs for the context rule.
fn remove_fingerprint(
    e: &Env,
    context_type: &ContextRuleType,
    signer_ids: &Vec<u32>,
    policy_ids: &Vec<u32>,
) {
    let fingerprint = compute_fingerprint(e, context_type, signer_ids, policy_ids);
    e.storage().persistent().remove(&SmartAccountStorageKey::Fingerprint(fingerprint));
}

/// Returns a list of signer IDs to their full [`Signer`] objects by
/// reading each [`SignerEntry`] from persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_ids` - The global signer IDs to resolve.
fn get_signers(e: &Env, signer_ids: &Vec<u32>) -> Vec<Signer> {
    Vec::from_iter(
        e,
        signer_ids.iter().map(|id| {
            let key = SmartAccountStorageKey::SignerData(id);
            get_persistent_entry::<SignerEntry>(e, &key).expect("signer entry to exist").signer
        }),
    )
}

/// Returns a list of policy IDs to their [`Address`] values by
/// reading each [`PolicyEntry`] from persistent storage.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `policy_ids` - The global policy IDs to resolve.
fn get_policies(e: &Env, policy_ids: &Vec<u32>) -> Vec<Address> {
    Vec::from_iter(
        e,
        policy_ids.iter().map(|policy_id| {
            let key = SmartAccountStorageKey::PolicyData(policy_id);
            get_persistent_entry::<PolicyEntry>(e, &key).expect("policy entry to exist").policy
        }),
    )
}

/// Helper function that tries to retrieve a persistent storage value.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `key` - The storage key to retrieve the value for.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(
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
