pub mod storage;
mod test;
use soroban_sdk::{auth::CustomAccountInterface, Address, Env, Map, String, Symbol, Val, Vec};
pub use storage::{
    add_context_rule, add_policy, add_signer, authenticate, get_context_rule, get_context_rules,
    get_validated_context, remove_context_rule, remove_policy, remove_signer,
    update_context_rule_name, update_context_rule_valid_until, ContextRule, ContextRuleType, Meta,
    Signatures, Signer, SmartAccountError,
};

pub trait SmartAccount: CustomAccountInterface {
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule;

    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule>;

    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule;

    fn update_context_rule_name(e: &Env, context_rule_id: u32, name: String) -> ContextRule;

    fn update_context_rule_valid_until(
        e: &Env,
        context_rule_id: u32,
        valid_until: Option<u32>,
    ) -> ContextRule;

    fn remove_context_rule(e: &Env, context_rule_id: u32);

    fn add_signer(e: &Env, context_rule_id: u32, signer: Signer);

    fn remove_signer(e: &Env, context_rule_id: u32, signer: Signer);

    fn add_policy(e: &Env, context_rule_id: u32, policy: Address, install_param: Val);

    fn remove_policy(e: &Env, context_rule_id: u32, policy: Address);
}

// Simple execution entry-point to call arbitrary contracts.
//
// Most likely to be used to call owned stateful polciies, and as direct
// contract-to-contract invocations are always authorized, that's a way to avoid
// re-entry when the policy need to auth back its owner.
pub trait ExecutionEntryPoint {
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

// ################## EVENTS ##################

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
pub fn emit_context_rule_added(e: &Env, context_rule: &ContextRule) {
    let topics = (Symbol::new(e, "context_rule_added"), context_rule.id);
    e.events().publish(
        topics,
        (
            context_rule.name.clone(),
            context_rule.context_type.clone(),
            context_rule.valid_until,
            context_rule.signers.clone(),
            context_rule.policies.clone(),
        ),
    )
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
pub fn emit_context_rule_updated(e: &Env, context_rule_id: u32, meta: &Meta) {
    let topics = (Symbol::new(e, "context_rule_updated"), context_rule_id);
    e.events().publish(topics, (meta.name.clone(), meta.context_type.clone(), meta.valid_until))
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
    let topics = (Symbol::new(e, "context_rule_removed"), context_rule_id);
    e.events().publish(topics, ())
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
pub fn emit_signer_added(e: &Env, context_rule_id: u32, signer: &Signer) {
    let topics = (Symbol::new(e, "signer_added"), context_rule_id);
    e.events().publish(topics, signer.clone())
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
pub fn emit_signer_removed(e: &Env, context_rule_id: u32, signer: &Signer) {
    let topics = (Symbol::new(e, "signer_removed"), context_rule_id);
    e.events().publish(topics, signer.clone())
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
pub fn emit_policy_added(e: &Env, context_rule_id: u32, policy: &Address, install_param: &Val) {
    let topics = (Symbol::new(e, "policy_added"), context_rule_id);
    e.events().publish(topics, (policy.clone(), install_param))
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
pub fn emit_policy_removed(e: &Env, context_rule_id: u32, policy: &Address) {
    let topics = (Symbol::new(e, "policy_removed"), context_rule_id);
    e.events().publish(topics, policy.clone())
}
