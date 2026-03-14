//! # Policy Building Blocks
//!
//! This module contains the core `Policy` trait and functions necessary to
//! implement some authorization policies for smart accounts. It provides
//! utility functions for `simple_threshold` (basic M-of-N multisig),
//! `weighted_threshold` (complex weighted voting), and `spending_limit`
//! (rolling window spending limits) that can be used to build policy contracts.
use soroban_sdk::{auth::Context, contractclient, Address, Env, FromVal, Val, Vec};

use crate::smart_account::{ContextRule, Signer};

pub mod simple_threshold;
pub mod spending_limit;
#[cfg(test)]
mod test;
pub mod weighted_threshold;

/// Core trait for authorization policies in smart accounts.
///
/// Policies define custom authorization logic that can be attached to context
/// rules. They provide flexible, programmable authorization beyond simple
/// signature verification, enabling complex business logic, spending limits,
/// time-based restrictions, and more.
///
/// # Lifecycle
///
/// Policies follow a three-phase lifecycle:
/// 1. **Installation** - Policy is configured and attached to a context rule.
/// 2. **Enforcement** - Policy validates and enforces authorization attempts.
/// 3. **Uninstallation** - Policy is removed and cleaned up.
///
/// # Type Parameters
///
/// * `AccountParams` - Installation parameters specific to the policy type.
///
/// # Sharing
///
/// Policies can be shared across multiple smart accounts or owned by only one,
/// depending on the implementation. Shared policies should handle multi-tenancy
/// appropriately in their storage design.
///
/// # Implementation Guidelines
///
/// - `enforce`: Performs both validation and state changes; must be authorized
///   by the smart account. Should panic if the policy conditions are not met.
/// - `install`/`uninstall`: Handle policy-specific setup and cleanup.
pub trait Policy {
    type AccountParams: FromVal<Env, Val>;

    /// Enforces the policy's authorization logic, performing both validation
    /// and any state changes.
    ///
    /// This method is called during authorization to verify that policy
    /// conditions are met. It should panic if the conditions are not
    /// satisfied. It can modify storage state as part of the authorization
    /// process and must be authorized by the smart account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context` - The authorization context being enforced.
    /// * `authenticated_signers` - List of signers that have been verified.
    /// * `context_rule` - The context rule this policy is attached to.
    /// * `smart_account` - The address of the smart account being authorized.
    ///
    /// # Authorization
    ///
    /// This method must be called with proper authorization from the smart
    /// account. Typically this means `smart_account.require_auth()` should
    /// be called before or during the execution of this method.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because enforcement logic is
    /// entirely policy-specific (e.g., threshold checks, spending limits,
    /// time restrictions). See [`simple_threshold`],
    /// [`weighted_threshold`], and [`spending_limit`] for reference
    /// implementations.
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    );

    /// Installs the policy for a specific context rule and smart account.
    ///
    /// This method is called when a policy is added to a context rule. It
    /// should initialize any necessary storage, validate installation
    /// parameters, and prepare the policy for enforcement.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `install_params` - Policy-specific installation parameters.
    /// * `context_rule` - The context rule this policy is being attached to.
    /// * `smart_account` - The address of the smart account installing this
    ///   policy.
    ///
    /// # Events
    ///
    /// Implementations should emit a policy-specific installed event
    /// containing the installation parameters, smart account address, and
    /// context rule ID. See [`simple_threshold`], [`weighted_threshold`], and
    /// [`spending_limit`] for reference implementations.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because installation logic is
    /// policy-specific (e.g., storing threshold parameters, initializing
    /// spending windows).
    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    );

    /// Uninstalls the policy from a context rule and cleans up associated data.
    ///
    /// This method is called when a policy is removed from a context rule. It
    /// should verify the policy is installed, clean up any storage, and prepare
    /// for the policy's removal. Implementations must panic if the policy is
    /// not installed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule` - The context rule this policy is being removed from.
    /// * `smart_account` - The address of the smart account uninstalling this
    ///   policy.
    ///
    /// # Events
    ///
    /// Implementations should emit a policy-specific uninstalled event
    /// containing the smart account address and context rule ID. See
    /// [`simple_threshold`], [`weighted_threshold`], and [`spending_limit`] for
    /// reference implementations.
    ///
    /// Note that the smart account calls `uninstall` via `try_uninstall`,
    /// so if uninstall panics, the policy's uninstalled event is rolled
    /// back while the smart account's `PolicyRemoved` event still fires.
    /// This is correct behavior — the absent policy event signals that
    /// cleanup did not complete cleanly.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because cleanup logic is
    /// policy-specific (e.g., removing threshold parameters, clearing
    /// spending windows).
    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address);
}

// A `PolicyClientInterface` must be declared here instead of using the public
// trait above, because traits with associated types are not supported by the
// `#[contractclient]` macro. While this may appear redundant, it is a
// necessary workaround: an identical internal trait is declared with the macro
// to generate the required client implementation. Interaction should occur
// through the public `Policy` trait above.
#[allow(unused)]
#[contractclient(name = "PolicyClient")]
trait PolicyClientInterface {
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    );

    fn install(e: &Env, install_params: Val, context_rule: ContextRule, smart_account: Address);

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address);
}
