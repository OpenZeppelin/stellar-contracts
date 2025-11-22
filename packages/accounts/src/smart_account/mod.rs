mod storage;
#[cfg(test)]
mod test;
use soroban_sdk::{
    auth::CustomAccountInterface, contractclient, contracterror, Address, Env, Map,
    String, Symbol, Val, Vec,
};

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::{contractevent};

pub use storage::{
    add_context_rule, add_policy, add_signer, authenticate, do_check_auth, get_context_rule,
    get_context_rules, get_validated_context, remove_context_rule, remove_policy, remove_signer,
    update_context_rule_name, update_context_rule_valid_until, ContextRule, ContextRuleType, Meta,
    Signatures, Signer,
};


#[cfg(feature = "certora")]
pub mod specs;


/// Core trait for smart account functionality, extending Soroban's
/// CustomAccountInterface with context rule management capabilities.
///
/// This trait provides methods for managing context rules, which define
/// authorization policies for different types of operations. Context rules can
/// contain signers and policies.
///
/// # Context Rules
///
/// Context rules are the fundamental building blocks of smart account
/// authorization:
/// - Each rule has a unique ID and applies to a specific context type
/// - Rules can contain multiple signers and policies
/// - Rules can have expiration times for temporary authorization
/// - Rules are validated against maximum limits (MAX_SIGNERS, MAX_POLICIES)
#[contractclient(name = "SmartAccountClient")]
pub trait SmartAccount: CustomAccountInterface {
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
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule;

    /// Retrieves all context rules of a specific type, returning a vector of
    /// all `ContextRule`s matching the specified type. Returns an empty
    /// vector if no rules of the given type exist.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_type` - The type of context rules to retrieve (e.g.,
    ///   Default, CallContract).
    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule>;

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
    /// * [`SmartAccountError::TooManyContextRules`] - When the number of
    ///   context rules exceeds MAX_CONTEXT_RULES (15).
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
    ///
    /// # Events
    ///
    /// * topics - `["context_rule_added", id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>, signers: Vec<Signer>, policies: Vec<Address>]`
    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule;

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
    /// * topics - `["context_rule_updated", context_rule_id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>]`
    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule;

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
    /// * topics - `["context_rule_updated", context_rule_id: u32]`
    /// * data - `[name: String, context_type: ContextRuleType, valid_until:
    ///   Option<u32>]`
    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule;

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
    fn remove_context_rule(e: &Env, context_rule_id: u32);

    /// Adds a new signer to an existing context rule.
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
    /// * data - `[signer: Signer]`
    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer);

    /// Removes a signer from an existing context rule. Removing the last signer
    /// is allowed only if the rule has at least one policy.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `signer` - The signer to remove from the context rule.
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
    /// * data - `[signer: Signer]`
    fn remove_signer(e: &Env, context_rule_id: u32, signer: Signer);

    /// Adds a new policy to an existing context rule and installs it. The
    /// policy's `install` method will be called during this operation.
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
    /// * data - `[policy: Address, install_param: Val]`
    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val);

    /// Removes a policy from an existing context rule and uninstalls it. The
    /// policy's `uninstall` method will be called during this operation.
    /// Removing the last policy is allowed only if the rule has at least
    /// one signer.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule_id` - The ID of the context rule to modify.
    /// * `policy` - The address of the policy contract to remove.
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
    /// * data - `[policy: Address]`
    fn remove_policy(e: &Env, context_rule_id: u32, policy: Address);
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
/// Implement this trait to enable your smart account to execute arbitrary
/// contract calls. This is particularly useful for:
/// - Calling owned policy contracts
/// - Interacting with external protocols on behalf of the smart account
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
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const SMART_ACCOUNT_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const SMART_ACCOUNT_TTL_THRESHOLD: u32 = SMART_ACCOUNT_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum number of policies allowed per context rule.
pub const MAX_POLICIES: u32 = 5;
/// Maximum number of signers allowed per context rule.
pub const MAX_SIGNERS: u32 = 15;
/// Maximum number of context rules allowed per smart account.
pub const MAX_CONTEXT_RULES: u32 = 15;

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
    /// Too many context rules in the smart account.
    TooManyContextRules = 3012,
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
    pub signers: Vec<Signer>,
    pub policies: Vec<Address>,
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
///   Option<u32>, signers: Vec<Signer>, policies: Vec<Address>]`
#[cfg(not(feature = "certora"))]
pub fn emit_context_rule_added(e: &Env, context_rule: &ContextRule) {
    ContextRuleAdded {
        context_rule_id: context_rule.id,
        name: context_rule.name.clone(),
        context_type: context_rule.context_type.clone(),
        valid_until: context_rule.valid_until,
        signers: context_rule.signers.clone(),
        policies: context_rule.policies.clone(),
    }
    .publish(e);
}

/// Event emitted when a context rule is updated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRuleUpdated {
    #[topic]
    pub context_rule_id: u32,
    pub name: String,
    pub context_type: ContextRuleType,
    pub valid_until: Option<u32>,
}

/// Emits an event indicating a context rule has been updated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the updated context rule.
/// * `meta` - The meta data of the context rule.
///
/// # Events
///
/// * topics - `["context_rule_updated", context_rule_id: u32]`
/// * data - `[name: String, context_type: ContextRuleType, valid_until:
///   Option<u32>]`
#[cfg(not(feature = "certora"))]
pub fn emit_context_rule_updated(e: &Env, context_rule_id: u32, meta: &Meta) {
    ContextRuleUpdated {
        context_rule_id,
        name: meta.name.clone(),
        context_type: meta.context_type.clone(),
        valid_until: meta.valid_until,
    }
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
#[cfg(not(feature = "certora"))]
pub fn emit_context_rule_removed(e: &Env, context_rule_id: u32) {
    ContextRuleRemoved { context_rule_id }.publish(e);
}

/// Event emitted when a signer is added to a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerAdded {
    #[topic]
    pub context_rule_id: u32,
    pub signer: Signer,
}

/// Emits an event indicating a signer has been added to a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `signer` - The signer that was added.
///
/// # Events
///
/// * topics - `["signer_added", context_rule_id: u32]`
/// * data - `[signer: Signer]`
#[cfg(not(feature = "certora"))]
pub fn emit_signer_added(e: &Env, context_rule_id: u32, signer: &Signer) {
    SignerAdded { context_rule_id, signer: signer.clone() }.publish(e);
}

/// Event emitted when a signer is removed from a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerRemoved {
    #[topic]
    pub context_rule_id: u32,
    pub signer: Signer,
}

/// Emits an event indicating a signer has been removed from a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `signer` - The signer that was removed.
///
/// # Events
///
/// * topics - `["signer_removed", context_rule_id: u32]`
/// * data - `[signer: Signer]`
#[cfg(not(feature = "certora"))]
pub fn emit_signer_removed(e: &Env, context_rule_id: u32, signer: &Signer) {
    SignerRemoved { context_rule_id, signer: signer.clone() }.publish(e);
}

/// Event emitted when a policy is added to a context rule.
#[contractevent]
#[derive(Clone, Debug)]
pub struct PolicyAdded {
    #[topic]
    pub context_rule_id: u32,
    pub policy: Address,
    pub install_param: Val,
}

/// Emits an event indicating a policy has been added to a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy` - The policy address that was added.
/// * `install_param` - The installation parameter for the policy.
///
/// # Events
///
/// * topics - `["policy_added", context_rule_id: u32]`
/// * data - `[policy: Address, install_param: Val]`
#[cfg(not(feature = "certora"))]
pub fn emit_policy_added(e: &Env, context_rule_id: u32, policy: &Address, install_param: Val) {
    PolicyAdded { context_rule_id, policy: policy.clone(), install_param }.publish(e);
}

/// Event emitted when a policy is removed from a context rule.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRemoved {
    #[topic]
    pub context_rule_id: u32,
    pub policy: Address,
}

/// Emits an event indicating a policy has been removed from a context rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the context rule.
/// * `policy` - The policy address that was removed.
///
/// # Events
///
/// * topics - `["policy_removed", context_rule_id: u32]`
/// * data - `[policy: Address]`
#[cfg(not(feature = "certora"))]
pub fn emit_policy_removed(e: &Env, context_rule_id: u32, policy: &Address) {
    PolicyRemoved { context_rule_id, policy: policy.clone() }.publish(e);
}


