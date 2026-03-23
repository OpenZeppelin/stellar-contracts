//! # Smart Account Storage - Context-Centric Authorization
//!
//! This module implements a flexible, context-centric authorization system for
//! smart accounts that separates concerns into three key dimensions:
//!
//! ## Architecture Overview
//!
//! ### Signers - Authentication
//! - **Delegated**: A Soroban `Address` that uses built-in signature
//!   verification via `require_auth_for_args`.
//! - **External**: A public key paired with a verifier contract for custom
//!   cryptographic verification (e.g., secp256r1, passkeys).
//!
//! ### Context Rules - Scope and Routing
//! - A context rule binds a set of signers and policies to a specific operation
//!   scope (`Default`, `CallContract(Address)`, or
//!   `CreateContract(BytesN<32>)`).
//! - Multiple rules can exist for the same scope with different signer sets and
//!   policies.
//! - Each rule must contain at least one signer or one policy, and can have an
//!   optional expiration (`valid_until`) defined by a ledger sequence.
//! - The caller explicitly selects which rule to validate against; no rule
//!   iteration or auto-discovery is performed.
//!
//! #### `soroban_sdk::auth::Context` and `ContextRule`
//!
//! `soroban_sdk::auth::Context` is a Soroban SDK type representing a single
//! authorized operation within a transaction. `__check_auth` receives
//! `auth_contexts: Vec<Context>` — one entry per `require_auth` call. Variants:
//! - `Contract(ContractContext)` — a contract function call (`contract`,
//!   `fn_name`, `args`)
//! - `CreateContractHostFn(CreateContractHostFnContext)` — contract deployment
//!   without constructor arguments (`executable`, `salt`)
//! - `CreateContractWithCtorHostFn(...)` — contract deployment with constructor
//!   arguments
//!
//! A [`ContextRule`] is this library's stored authorization entry, bound to a
//! [`ContextRuleType`] that narrows which `Context` variants it can authorize.
//! A smart account can hold **multiple [`ContextRule`]s for the same context
//! type** — for example, an admin rule and a session rule both scoped to the
//! same contract.
//!
//! During authorization the caller must supply **exactly one [`ContextRule`] ID
//! per `Context`** via [`AuthPayload::context_rule_ids`]. No iteration is
//! performed; the caller explicitly selects which stored rule to validate
//! against for each operation.
//!
//! ### Policies - Enforcement Logic
//! - External contracts attached to context rules that enforce business
//!   constraints (e.g., spending limits, threshold multisig).
//! - All policies in a rule must be satisfied (all-or-nothing enforcement).
//!
//! ## Key Design Principles
//!
//! ### Context-Centric Approach
//! The system flips traditional key-centric reasoning to focus on **what is
//! being authorized** rather than **which keys are signing**. This mirrors
//! familiar web2 OAuth patterns where the primary focus is on the
//! scope/permissions being granted, not the underlying keys.
//!
//! ### Multiple Rules Per Context
//! Different authorization requirements for the same context:
//! - Admin config: 2-of-3 threshold for contract calls
//! - User config: 3-of-5 threshold for the same contract calls
//! - Emergency config: 1-of-1 with additional policy constraints
//!
//! ## Authorization Matching Algorithm
//!
//! The caller explicitly selects which rule to use for each context via
//! [`AuthPayload::context_rule_ids`], a vector aligned by index with the
//! `auth_contexts` passed to `__check_auth`. No rule iteration or
//! auto-discovery is performed.
//!
//! For each (context, rule_id) pair:
//!
//! I. Authenticate all provided signatures (delegated and external).
//! II. For each context, look up the rule by its explicit ID:
//!     1. Reject if the rule is expired.
//!     2. Reject if the rule's context type does not match the actual context
//!        (a `Default` rule matches any context).
//!     3. Identify authenticated signers out of all provided signers.
//!     4.a. If the rule has no policies, all rule signers must be
//!          authenticated — otherwise reject.
//!     4.b. If the rule has policies, defer full signer validation to each
//!          policy's `enforce()` call.
//! III. Enforce all policies for every validated (rule, context) pair.
//! IV. If any check fails, authorization fails.
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

mod storage;
#[cfg(test)]
mod test;
use soroban_sdk::{
    auth::CustomAccountInterface, contracterror, contractevent, contracttrait, Address, Env, Map,
    String, Symbol, Val, Vec,
};
pub use storage::{
    add_context_rule, add_policy, add_signer, authenticate, batch_add_signer,
    contains_canonical_duplicate, do_check_auth, get_context_rule, get_context_rules_count,
    get_validated_context_by_id, remove_context_rule, remove_policy, remove_signer,
    update_context_rule_name, update_context_rule_valid_until, validate_signer_key_size,
    AuthPayload, ContextRule, ContextRuleEntry, ContextRuleType, Signer, SmartAccountStorageKey,
};

/// Core trait for smart account functionality, extending Soroban's
/// CustomAccountInterface with context rule management capabilities.
///
/// This trait provides methods for managing context rules, which define
/// authorization policies for different types of operations. Context rules can
/// contain signers and policies.
#[contracttrait]
pub trait SmartAccount: CustomAccountInterface {
    /// Retrieves the number of all context rules, including expired rules.
    /// Defaults to 0.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn get_context_rules_count(e: &Env) -> u32 {
        storage::get_context_rules_count(e)
    }

    /// Retrieves a context rule by its unique ID, returning the
    /// `ContextRule` containing all metadata, signers, and policies.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The unique identifier of the context rule to
    ///   retrieve.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule {
        storage::get_context_rule(e, context_rule_id)
    }

    /// Creates a new context rule with the specified configuration, returning
    /// the newly created `ContextRule` with a unique ID assigned. Installs
    /// all specified policies during creation.
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
    /// * [`SmartAccountError::NoSignersAndPolicies`] - When both signers and
    ///   policies are empty.
    /// * [`SmartAccountError::TooManySigners`] - When signers exceed
    ///   MAX_SIGNERS (15).
    /// * [`SmartAccountError::TooManyPolicies`] - When policies exceed
    ///   MAX_POLICIES (5).
    /// * [`SmartAccountError::DuplicateSigner`] - When the same signer appears
    ///   multiple times.
    /// * [`SmartAccountError::PastValidUntil`] - When valid_until is in the
    ///   past.
    /// * [`SmartAccountError::MathOverflow`] - When the context rule, signer,
    ///   or policy ID counter has reached `u32::MAX`.
    ///
    /// # Events
    ///
    /// * topics - `["context_rule_added", id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>, signer_ids: Vec<u32>, policy_ids: Vec<u32>]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::add_context_rule`].
    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();
        storage::add_context_rule(e, &context_type, &name, valid_until, &signers, &policies)
    }

    /// Updates the name of an existing context rule, returning the updated
    /// `ContextRule` with the new name.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to update.
    /// * `name` - The new human-readable name for the context rule.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    ///
    /// # Events
    ///
    /// * topics - `["context_rule_meta_updated", context_rule_id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::update_context_rule_name`].
    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule {
        e.current_contract_address().require_auth();
        storage::update_context_rule_name(e, context_rule_id, &name)
    }

    /// Updates the expiration time of an existing context rule, returning the
    /// updated `ContextRule` with the new expiration time.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to update.
    /// * `valid_until` - New optional expiration ledger sequence. Use `None`
    ///   for no expiration.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    /// * [`SmartAccountError::PastValidUntil`] - When valid_until is in the
    ///   past.
    ///
    /// # Events
    ///
    /// * topics - `["context_rule_meta_updated", context_rule_id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::update_context_rule_valid_until`].
    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();
        storage::update_context_rule_valid_until(e, context_rule_id, valid_until)
    }

    /// Removes a context rule and cleans up all associated data. This function
    /// uninstalls all policies associated with the rule and removes all stored
    /// data including signers, policies, and metadata.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to remove.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    ///
    /// # Events
    ///
    /// * topics - `["context_rule_removed", context_rule_id: u32]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::remove_context_rule`].
    fn remove_context_rule(e: &Env, context_rule_id: u32) {
        e.current_contract_address().require_auth();
        storage::remove_context_rule(e, context_rule_id);
    }

    /// Adds a new signer to an existing context rule, returning the assigned
    /// signer ID.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `signer` - The signer to add to the context rule.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    /// * [`SmartAccountError::DuplicateSigner`] - When the signer already
    ///   exists in the rule.
    /// * [`SmartAccountError::TooManySigners`] - When adding would exceed
    ///   MAX_SIGNERS (15).
    ///
    /// # Events
    ///
    /// * topics - `["signer_added", context_rule_id: u32]`
    /// * data - `[signer_id: u32]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::add_signer`].
    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer) -> u32 {
        e.current_contract_address().require_auth();
        storage::add_signer(e, context_rule_id, &signer)
    }

    /// Removes a signer from an existing context rule. Removing the last signer
    /// is allowed only if the rule has at least one policy.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `signer_id` - The ID of the signer to remove from the context rule.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    /// * [`SmartAccountError::SignerNotFound`] - When the signer doesn't exist
    ///   in the rule.
    ///
    /// # Events
    ///
    /// * topics - `["signer_removed", context_rule_id: u32]`
    /// * data - `[signer_id: u32]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::remove_signer`].
    fn remove_signer(e: &Env, context_rule_id: u32, signer_id: u32) {
        e.current_contract_address().require_auth();
        storage::remove_signer(e, context_rule_id, signer_id);
    }

    /// Adds a new policy to an existing context rule, installs it, and returns
    /// the assigned policy ID. The policy's `install` method will be called
    /// during this operation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `policy` - The address of the policy contract to add.
    /// * `install_param` - The installation parameter for the policy.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    /// * [`SmartAccountError::DuplicatePolicy`] - When the policy already
    ///   exists in the rule.
    /// * [`SmartAccountError::TooManyPolicies`] - When adding would exceed
    ///   MAX_POLICIES (5).
    ///
    /// # Events
    ///
    /// * topics - `["policy_added", context_rule_id: u32]`
    /// * data - `[policy_id: u32]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::add_policy`].
    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val) -> u32 {
        e.current_contract_address().require_auth();
        storage::add_policy(e, context_rule_id, &policy, install_param)
    }

    /// Removes a policy from an existing context rule and uninstalls it. The
    /// policy's `uninstall` method will be called during this operation.
    /// Removing the last policy is allowed only if the rule has at least
    /// one signer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `policy_id` - The ID of the policy to remove from the context rule.
    ///
    /// # Errors
    ///
    /// * [`SmartAccountError::ContextRuleNotFound`] - When no context rule
    ///   exists with the given ID.
    /// * [`SmartAccountError::PolicyNotFound`] - When the policy doesn't exist
    ///   in the rule.
    ///
    /// # Events
    ///
    /// * topics - `["policy_removed", context_rule_id: u32]`
    /// * data - `[policy_id: u32]`
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then delegating to
    /// [`storage::remove_policy`].
    fn remove_policy(e: &Env, context_rule_id: u32, policy_id: u32) {
        e.current_contract_address().require_auth();
        storage::remove_policy(e, context_rule_id, policy_id);
    }
}

/// Simple execution entry-point to call arbitrary contracts from within a smart
/// account.
///
/// # Security Considerations
///
/// Since direct contract-to-contract invocations are always authorized in
/// Soroban, this trait provides a way to avoid re-entry issues when policies
/// need to authenticate back to their owner smart account.
///
/// # Usage
///
/// Implement this trait to enable a smart account to execute arbitrary
/// contract calls. This is particularly useful for:
/// - Calling owned policy contracts
/// - Interacting with external protocols on behalf of the smart account
#[contracttrait]
pub trait ExecutionEntryPoint {
    /// Executes a function call on a target contract from within the smart
    /// account context.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `target` - The address of the contract to call.
    /// * `target_fn` - The function name to invoke on the target contract.
    /// * `target_args` - Arguments to pass to the target function.
    ///
    /// # Notes
    ///
    /// Defaults to requiring authorization from the smart account itself
    /// (`e.current_contract_address().require_auth()`) and then calling
    /// `e.invoke_contract()`.
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>) {
        e.current_contract_address().require_auth();
        e.invoke_contract::<Val>(&target, &target_fn, target_args);
    }
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const SMART_ACCOUNT_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const SMART_ACCOUNT_TTL_THRESHOLD: u32 = SMART_ACCOUNT_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum number of policies allowed per context rule.
pub const MAX_POLICIES: u32 = 5;
/// Maximum number of signers allowed per context rule.
pub const MAX_SIGNERS: u32 = 15;
/// Maximum length in bytes for a context rule name.
pub const MAX_NAME_SIZE: u32 = 20;
/// Maximum size in bytes for external signer key data.
pub const MAX_EXTERNAL_KEY_SIZE: u32 = 256;

// ################## ERRORS ##################

/// Error codes for smart account operations.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SmartAccountError {
    /// The specified context rule does not exist.
    ContextRuleNotFound = 3000,
    /// A duplicate context rule already exists.
    DuplicateContextRule = 3001,
    /// The provided context cannot be validated against any rule.
    UnvalidatedContext = 3002,
    /// External signature verification failed.
    ExternalVerificationFailed = 3003,
    /// Context rule must have at least one signer or policy.
    NoSignersAndPolicies = 3004,
    /// The valid_until timestamp is in the past.
    PastValidUntil = 3005,
    /// The specified signer was not found.
    SignerNotFound = 3006,
    /// The signer already exists in the context rule.
    DuplicateSigner = 3007,
    /// The specified policy was not found.
    PolicyNotFound = 3008,
    /// The policy already exists in the context rule.
    DuplicatePolicy = 3009,
    /// Too many signers in the context rule.
    TooManySigners = 3010,
    /// Too many policies in the context rule.
    TooManyPolicies = 3011,
    /// An internal ID counter (context rule, signer, or policy) has reached
    /// its maximum value (`u32::MAX`) and cannot be incremented further.
    MathOverflow = 3012,
    /// External signer key data exceeds the maximum allowed size.
    KeyDataTooLarge = 3013,
    /// context_rule_ids length does not match auth_contexts length.
    ContextRuleIdsLengthMismatch = 3014,
    /// Context rule name exceeds the maximum allowed length.
    NameTooLong = 3015,
}

// ################## EVENTS ##################

/// Event emitted when a context rule is added.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRuleAdded {
    #[topic]
    pub context_rule_id: u32,
    pub name: String,
    pub context_type: ContextRuleType,
    pub valid_until: Option<u32>,
    pub signer_ids: Vec<u32>,
    pub policy_ids: Vec<u32>,
}

/// Emits an event indicating a context rule has been added.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The newly created context rule.
///
/// # Events
///
/// * topics - `["context_rule_added", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>, signer_ids: Vec<u32>, policy_ids: Vec<u32>]`
pub fn emit_context_rule_added(
    e: &Env,
    context_rule_id: u32,
    name: &String,
    context_type: &ContextRuleType,
    valid_until: Option<u32>,
    signer_ids: &Vec<u32>,
    policy_ids: &Vec<u32>,
) {
    ContextRuleAdded {
        context_rule_id,
        name: name.clone(),
        context_type: context_type.clone(),
        valid_until,
        signer_ids: signer_ids.clone(),
        policy_ids: policy_ids.clone(),
    }
    .publish(e);
}

/// Event emitted when a context rule name or valid_until are updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRuleMetaUpdated {
    #[topic]
    pub context_rule_id: u32,
    pub name: String,
    pub valid_until: Option<u32>,
}

/// Emits an event indicating a context rule name or valid_until have been
/// updated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the updated context rule.
/// * `name` - The name of the context rule.
/// * `valid_until` - The validity of the context rule.
///
/// # Events
///
/// * topics - `["context_rule_updated", context_rule_id: u32]`
/// * data - `[name: String, valid_until: Option<u32>]`
pub fn emit_context_rule_meta_updated(
    e: &Env,
    context_rule_id: u32,
    name: &String,
    valid_until: &Option<u32>,
) {
    ContextRuleMetaUpdated { context_rule_id, name: name.clone(), valid_until: *valid_until }
        .publish(e);
}

/// Event emitted when a context rule is removed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRuleRemoved {
    #[topic]
    pub context_rule_id: u32,
}

/// Emits an event indicating a context rule has been removed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the removed context rule.
///
/// # Events
///
/// * topics - `["context_rule_removed", context_rule_id: u32]`
/// * data - `[]`
pub fn emit_context_rule_removed(e: &Env, context_rule_id: u32) {
    ContextRuleRemoved { context_rule_id }.publish(e);
}

/// Event emitted when a signer is added to a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerAdded {
    #[topic]
    pub context_rule_id: u32,
    pub signer_id: u32,
}

/// Emits an event indicating a signer has been added to a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `signer_id` - The signer ID that was added.
///
/// # Events
///
/// * topics - `["signer_added", context_rule_id: u32]`
/// * data - `[signer_id: u32]`
pub fn emit_signer_added(e: &Env, context_rule_id: u32, signer_id: u32) {
    SignerAdded { context_rule_id, signer_id }.publish(e);
}

/// Event emitted when a signer is removed from a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRemoved {
    #[topic]
    pub context_rule_id: u32,
    pub signer_id: u32,
}

/// Emits an event indicating a signer has been removed from a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `signer_id` - The signer ID that was removed.
///
/// # Events
///
/// * topics - `["signer_removed", context_rule_id: u32]`
/// * data - `[signer_id: u32]`
pub fn emit_signer_removed(e: &Env, context_rule_id: u32, signer_id: u32) {
    SignerRemoved { context_rule_id, signer_id }.publish(e);
}

/// Event emitted when a policy is added to a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyAdded {
    #[topic]
    pub context_rule_id: u32,
    pub policy_id: u32,
}

/// Emits an event indicating a policy has been added to a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy_id` - The policy ID that was added.
///
/// # Events
///
/// * topics - `["policy_added", context_rule_id: u32]`
/// * data - `[policy_id: u32]`
pub fn emit_policy_added(e: &Env, context_rule_id: u32, policy_id: u32) {
    PolicyAdded { context_rule_id, policy_id }.publish(e);
}

/// Event emitted when a policy is removed from a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRemoved {
    #[topic]
    pub context_rule_id: u32,
    pub policy_id: u32,
}

/// Emits an event indicating a policy has been removed from a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy_id` - The policy ID that was removed.
///
/// # Events
///
/// * topics - `["policy_removed", context_rule_id: u32]`
/// * data - `[policy_id: u32]`
pub fn emit_policy_removed(e: &Env, context_rule_id: u32, policy_id: u32) {
    PolicyRemoved { context_rule_id, policy_id }.publish(e);
}

/// Event emitted when a signer is registered in the global registry.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRegistered {
    #[topic]
    pub signer_id: u32,
    pub signer: Signer,
}

/// Emits an event indicating a signer has been registered in the global
/// registry.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_id` - The ID assigned to the signer.
/// * `signer` - The signer that was registered.
///
/// # Events
///
/// * topics - `["signer_registered", signer_id: u32]`
/// * data - `[signer: Signer]`
pub fn emit_signer_registered(e: &Env, signer_id: u32, signer: &Signer) {
    SignerRegistered { signer_id, signer: signer.clone() }.publish(e);
}

/// Event emitted when a signer is deregistered from the global registry.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerDeregistered {
    #[topic]
    pub signer_id: u32,
}

/// Emits an event indicating a signer has been deregistered from the global
/// registry.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signer_id` - The ID of the signer that was deregistered.
///
/// # Events
///
/// * topics - `["signer_deregistered", signer_id: u32]`
/// * data - `[]`
pub fn emit_signer_deregistered(e: &Env, signer_id: u32) {
    SignerDeregistered { signer_id }.publish(e);
}

/// Event emitted when a policy is registered in the global registry.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRegistered {
    #[topic]
    pub policy_id: u32,
    pub policy: Address,
}

/// Emits an event indicating a policy has been registered in the global
/// registry.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `policy_id` - The ID assigned to the policy.
/// * `policy` - The policy address that was registered.
///
/// # Events
///
/// * topics - `["policy_registered", policy_id: u32]`
/// * data - `[policy: Address]`
pub fn emit_policy_registered(e: &Env, policy_id: u32, policy: &Address) {
    PolicyRegistered { policy_id, policy: policy.clone() }.publish(e);
}

/// Event emitted when a policy is deregistered from the global registry.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDeregistered {
    #[topic]
    pub policy_id: u32,
}

/// Emits an event indicating a policy has been deregistered from the global
/// registry.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `policy_id` - The ID of the policy that was deregistered.
///
/// # Events
///
/// * topics - `["policy_deregistered", policy_id: u32]`
/// * data - `[]`
pub fn emit_policy_deregistered(e: &Env, policy_id: u32) {
    PolicyDeregistered { policy_id }.publish(e);
}
