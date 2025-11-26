//! # Policy Building Blocks
//!
//! This module contains the core `Policy` trait and functions necessary to
//! implement some authorization policies for smart accounts. It provides
//! utility functions for `simple_threshold` (basic M-of-N multisig),
//! `weighted_threshold` (complex weighted voting), and `spending_limit`
//! (rolling window spending limits) that can be used to build policy contracts.
use soroban_sdk::{auth::Context, contractclient, Address, Env, FromVal, Val, Vec};

use crate::smart_account::{ContextRule, Signer};

#[cfg(feature = "certora")]
pub mod specs;

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
/// 2. **Enforcement** - Policy validates and potentially modifies authorization
///    attempts.
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
/// - `can_enforce`: Should be pure validation with no state changes
/// - `enforce`: Can modify state and must be authorized by the smart account
/// - `install`/`uninstall`: Handle policy-specific setup and cleanup
///
/// # Examples
///
/// ```rust
/// use soroban_sdk::{Address, Env, Val};
/// use stellar_accounts::policies::Policy;
///
/// struct SpendingLimitPolicy;
/// impl Policy for SpendingLimitPolicy {
///     type AccountParams = u64;
///
///     // Daily spending limit
///
///     fn can_enforce(/* ... */) -> bool {
///         // Check if spending limit allows this transaction
///         true
///     }
///
///     fn enforce(/* ... */) {
///         // Update spending tracking
///     }
///
///     fn install(/* ... */) {
///         // Initialize spending limit storage
///     }
///
///     fn uninstall(/* ... */) {
///         // Clean up policy storage
///     }
/// }
/// ```
pub trait Policy {
    type AccountParams: FromVal<Env, Val>;

    /// Determines whether this policy can be enforced for the given
    /// authorization context.
    ///
    /// This method performs read-only validation to check if the policy's
    /// conditions are satisfied. It should not trigger any state changes as
    /// it may be called multiple times during authorization evaluation for
    /// different context rule configurations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context` - The authorization context being evaluated.
    /// * `authenticated_signers` - List of signers that have been
    ///   cryptographically verified.
    /// * `context_rule` - The context rule this policy is attached to.
    /// * `smart_account` - The address of the smart account being authorized.
    ///
    /// # Implementation Notes
    ///
    /// - Must be idempotent and side-effect free
    /// - May read from storage but must not modify it
    /// - Should be efficient as it may be called multiple times
    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool;

    /// Enforces the policy's authorization logic and may trigger state changes.
    ///
    /// This method serves as a hook that executes after `can_enforce` returns
    /// `true`. It can modify storage state and perform actions as part of
    /// the authorization process. This method must be authorized by the
    /// smart account.
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
    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    );

    /// Uninstalls the policy from a context rule and cleans up associated data.
    ///
    /// This method is called when a policy is removed from a context rule. It
    /// should clean up any storage, and prepare for the policy's removal.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `context_rule` - The context rule this policy is being removed from.
    /// * `smart_account` - The address of the smart account uninstalling this
    ///   policy.
    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address);
}

// We need to declare a `PolicyClientInterface` here, instead of using the
// public trait above, because traits with associated types are not supported
// by the `#[contractclient]` macro. While this may appear redundant, it's a
// necessary workaround: we declare an identical internal trait with the macro
// to generate the required client implementation. Users should only interact
// with the public `Policy` trait above for their implementations.
#[allow(unused)]
#[contractclient(name = "PolicyClient")]
pub trait PolicyClientInterface {
    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool;

    // this serves as a hook and can trigger state changes and must be authorized by
    // the smart account (`source.require_auth()`)
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
