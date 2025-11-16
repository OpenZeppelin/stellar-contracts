//! # Spending Limit Policy Contract
//!
//! A reusable policy contract that implements spending limit functionality.
//! This contract can be deployed once and used by multiple smart accounts,
//! with each account defining its own spending limit and time period for
//! different context rules. Enables transaction amount restrictions over
//! rolling time windows to prevent unauthorized large transactions.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // Set a spending limit of 10,000,000 stroops (10 XLM) over 1 day (17280 ledgers)
//! SpendingLimitAccountParams {
//!     spending_limit: 10_000_000, // 10 XLM in stroops
//!     period_ledgers: 17280,      // ~1 day in ledgers
//! }
//! ```
use soroban_sdk::{auth::Context, contract, contractimpl, Address, Env, Vec};
use stellar_accounts::{
    policies::{spending_limit, Policy},
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct SpendingLimitPolicyContract;

#[contractimpl]
impl Policy for SpendingLimitPolicyContract {
    type AccountParams = spending_limit::SpendingLimitAccountParams;

    /// Check if the spending limit policy can be enforced.
    ///
    /// Verifies that the transaction amount does not exceed the remaining
    /// spending limit for the current time period.
    ///
    /// # Returns
    ///
    /// * `true` if the spending limit allows the transaction
    /// * `false` otherwise
    fn can_enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> bool {
        spending_limit::can_enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        )
    }

    /// Enforce the spending limit policy.
    ///
    /// Records the transaction amount and updates the spending history.
    /// This is called after `can_enforce` returns true.
    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        spending_limit::enforce(e, &context, &authenticated_signers, &context_rule, &smart_account)
    }

    /// Install the spending limit policy for a smart account.
    ///
    /// Stores the spending limit configuration for the given context rule.
    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        spending_limit::install(e, &install_params, &context_rule, &smart_account)
    }

    /// Uninstall the spending limit policy for a smart account.
    ///
    /// Removes the spending limit configuration and history for the given
    /// context rule.
    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        spending_limit::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl SpendingLimitPolicyContract {
    /// Get the current spending limit data for a smart account
    pub fn get_spending_limit_data(
        e: Env,
        context_rule_id: u32,
        smart_account: Address,
    ) -> spending_limit::SpendingLimitData {
        spending_limit::get_spending_limit_data(&e, context_rule_id, &smart_account)
    }

    /// Set a new spending limit for a smart account
    pub fn set_spending_limit(
        e: Env,
        spending_limit: i128,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        spending_limit::set_spending_limit(&e, spending_limit, &context_rule, &smart_account)
    }
}
