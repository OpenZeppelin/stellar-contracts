//! # Spending Limit Policy Module
//!
//! This policy implements spending limit functionality where transactions above
//! the specified amount are blocked. It intersects transfer operations and
//! enforces spending limits over a configurable rolling time window.
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
use cvlr::nondet::Nondet;
use soroban_sdk::{
    auth::{Context, ContractContext},
    contracterror, contracttype, panic_with_error, symbol_short, Address, Env,
    TryFromVal, Vec,
};

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::{contractevent};

use crate::smart_account::{ContextRule, Signer};

/// Event emitted when a spending limit policy is enforced.
#[contractevent]
#[derive(Clone)]
pub struct SpendingLimitPolicyEnforced {
    #[topic]
    pub smart_account: Address,
    pub context: Context,
    pub context_rule_id: u32,
    pub amount: i128,
    pub total_spent_in_period: i128,
}

/// Installation parameters for the spending limit policy.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingLimitAccountParams {
    /// The maximum amount that can be spent within the specified period (in
    /// stroops).
    pub spending_limit: i128,
    /// The period in ledgers over which the spending limit applies.
    pub period_ledgers: u32,
}

impl Nondet for SpendingLimitAccountParams {
    fn nondet() -> Self {
        SpendingLimitAccountParams {
            spending_limit: i128::nondet(),
            period_ledgers: u32::nondet(),
        }
    }
}

/// Internal storage structure for spending limit tracking.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingLimitData {
    /// The spending limit for the period.
    pub spending_limit: i128,
    /// The period in ledgers over which the spending limit applies.
    pub period_ledgers: u32,
    /// History of spending transactions with their ledger sequences.
    pub spending_history: Vec<SpendingEntry>,
    /// Cached total of all amounts in spending_history.
    pub cached_total_spent: i128,
}

/// Individual spending entry for tracking purposes.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingEntry {
    /// The amount spent in this transaction.
    pub amount: i128,
    /// The ledger sequence when this transaction occurred.
    pub ledger_sequence: u32,
}

/// Error codes for spending limit policy operations.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SpendingLimitError {
    /// The smart account does not have a spending limit policy installed.
    SmartAccountNotInstalled = 3220,
    /// The spending limit has been exceeded.
    SpendingLimitExceeded = 3221,
    /// The spending limit or period is invalid.
    InvalidLimitOrPeriod = 3222,
    /// The transaction is not allowed by this policy.
    NotAllowed = 3223,
    /// The spending history has reached maximum capacity.
    HistoryCapacityExceeded = 3224,
}

/// Storage keys for spending limit policy data.
#[contracttype]
pub enum SpendingLimitStorageKey {
    /// Storage key for spending limit data of a smart account context rule.
    AccountContext(Address, u32),
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const SPENDING_LIMIT_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const SPENDING_LIMIT_TTL_THRESHOLD: u32 = SPENDING_LIMIT_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum number of spending entries to keep in history.
/// This prevents storage DoS by capping the vector size.
pub const MAX_HISTORY_ENTRIES: u32 = 1000;

// ################## QUERY STATE ##################

/// Retrieves the spending limit data for a smart account's spending limit
/// policy.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The context rule ID for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SpendingLimitError::SmartAccountNotInstalled`] - When the smart account
///   does not have a spending limit policy installed.
pub fn get_spending_limit_data(
    e: &Env,
    context_rule_id: u32,
    smart_account: &Address,
) -> SpendingLimitData {
    let key = SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule_id);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                SPENDING_LIMIT_TTL_THRESHOLD,
                SPENDING_LIMIT_EXTEND_AMOUNT,
            );
        })
        .unwrap_or_else(|| panic_with_error!(e, SpendingLimitError::SmartAccountNotInstalled))
}

/// Checks if the spending limit policy can be enforced for the given
/// transaction. Returns `true` if the transaction amount is within the spending
/// limit for the rolling period and there is at least one authenticated signer,
/// `false` otherwise or if the policy is not installed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `authenticated_signers` - The list of authenticated signers.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
pub fn can_enforce(
    e: &Env,
    context: &Context,
    authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) -> bool {
    if authenticated_signers.is_empty() {
        return false;
    }

    let key = SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let spending_data: Option<SpendingLimitData> = e.storage().persistent().get(&key);

    if let Some(data) = spending_data {
        e.storage().persistent().extend_ttl(
            &key,
            SPENDING_LIMIT_TTL_THRESHOLD,
            SPENDING_LIMIT_EXTEND_AMOUNT,
        );

        // Check if this is a contract call context
        match context {
            Context::Contract(ContractContext { fn_name, args, .. }) => {
                // Only enforce on transfer functions
                if fn_name == &symbol_short!("transfer") {
                    // Try to extract the amount from the third argument (index 2)
                    if let Some(amount_val) = args.get(2) {
                        if let Ok(amount) = i128::try_from_val(e, &amount_val) {
                            let current_ledger = e.ledger().sequence();
                            let cutoff_ledger = current_ledger.saturating_sub(data.period_ledgers);

                            // Calculate how much would be removed by cleanup
                            let mut expired_total = 0i128;
                            for (index, entry) in data.spending_history.iter().enumerate() {
                                if entry.ledger_sequence < cutoff_ledger {
                                    expired_total += entry.amount;
                                } else {
                                    // Check if adding this transaction would exceed history
                                    // capacity
                                    let remaining_entries =
                                        data.spending_history.len() - index as u32;
                                    if remaining_entries >= MAX_HISTORY_ENTRIES {
                                        return false;
                                    }
                                    break;
                                }
                            }

                            let total_spent = data.cached_total_spent - expired_total;

                            return total_spent + amount <= data.spending_limit;
                        }
                    }
                }
                // For non-transfer contract calls, policy is not valid
                false
            }
            _ => {
                // For non-contract call contexts, policy is not valid
                false
            }
        }
    } else {
        false
    }
}

// ################## CHANGE STATE ##################

/// Enforces the spending limit policy and updates the spending history.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `authenticated_signers` - The list of authenticated signers.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SpendingLimitError::SpendingLimitExceeded`] - When the transaction
///   amount is not within the spending limit for the rolling period.
/// * [`SpendingLimitError::NotAllowed`] - When there are no authenticated
///   signers, the context is not a transfer with well-formatted amount.
///
/// # Events
///
/// * topics - `["spending_limit_policy_enforced", smart_account: Address]`
/// * data - `[context: Context, context_rule_id: u32, amount: i128,
///   total_spent_in_period: i128]`
pub fn enforce(
    e: &Env,
    context: &Context,
    authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if authenticated_signers.is_empty() {
        panic_with_error!(e, SpendingLimitError::NotAllowed)
    }

    let key = SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let mut data = get_spending_limit_data(e, context_rule.id, smart_account);
    let current_ledger = e.ledger().sequence();

    match context {
        Context::Contract(ContractContext { fn_name, args, .. }) => {
            if fn_name == &symbol_short!("transfer") {
                if let Some(amount_val) = args.get(2) {
                    if let Ok(amount) = i128::try_from_val(e, &amount_val) {
                        // Clean up old entries outside the rolling window BEFORE checking limit
                        let removed_amount = cleanup_old_entries(
                            &mut data.spending_history,
                            current_ledger,
                            data.period_ledgers,
                        );
                        data.cached_total_spent -= removed_amount;

                        // Now check if the transaction exceeds the spending limit using updated
                        // cached total
                        if data.cached_total_spent + amount > data.spending_limit {
                            panic_with_error!(e, SpendingLimitError::SpendingLimitExceeded)
                        }

                        if data.spending_history.len() >= MAX_HISTORY_ENTRIES {
                            panic_with_error!(e, SpendingLimitError::HistoryCapacityExceeded)
                        }

                        // Add the new spending entry
                        let new_entry = SpendingEntry { amount, ledger_sequence: current_ledger };
                        data.spending_history.push_back(new_entry);
                        data.cached_total_spent += amount;

                        e.storage().persistent().set(&key, &data);

                        #[cfg(not(feature = "certora"))]
                        SpendingLimitPolicyEnforced {
                            smart_account: smart_account.clone(),
                            context: context.clone(),
                            context_rule_id: context_rule.id,
                            amount,
                            total_spent_in_period: data.cached_total_spent,
                        }
                        .publish(e);

                        return;
                    }
                }
            }
        }
        _ => {
            panic_with_error!(e, SpendingLimitError::NotAllowed)
        }
    }
    panic_with_error!(e, SpendingLimitError::NotAllowed)
}

/// Sets the spending limit for a smart account's spending limit policy.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spending_limit` - The new spending limit.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SpendingLimitError::InvalidLimitOrPeriod`] - When spending_limit is not
///   positive.
pub fn set_spending_limit(
    e: &Env,
    spending_limit: i128,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if spending_limit <= 0 {
        panic_with_error!(e, SpendingLimitError::InvalidLimitOrPeriod)
    }

    let key = SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule.id);
    let mut data = get_spending_limit_data(e, context_rule.id, smart_account);
    data.spending_limit = spending_limit;

    e.storage().persistent().set(&key, &data);
}

/// Installs the spending limit policy on a smart account.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - Installation parameters containing the spending limit and
///   period.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SpendingLimitError::InvalidLimitOrPeriod`] - When spending_limit is not
///   positive or period_ledgers is zero.
pub fn install(
    e: &Env,
    params: &SpendingLimitAccountParams,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    if params.spending_limit <= 0 || params.period_ledgers == 0 {
        panic_with_error!(e, SpendingLimitError::InvalidLimitOrPeriod)
    }

    let data = SpendingLimitData {
        spending_limit: params.spending_limit,
        period_ledgers: params.period_ledgers,
        spending_history: Vec::new(e),
        cached_total_spent: 0,
    };

    e.storage().persistent().set(
        &SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule.id),
        &data,
    );
}

/// Uninstalls the spending limit policy from a smart account.
/// Removes all stored spending limit data for the account and context rule.
/// Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The context rule for this policy.
/// * `smart_account` - The address of the smart account.
pub fn uninstall(e: &Env, context_rule: &ContextRule, smart_account: &Address) {
    // Require authorization from the smart_account
    smart_account.require_auth();

    e.storage()
        .persistent()
        .remove(&SpendingLimitStorageKey::AccountContext(smart_account.clone(), context_rule.id));
}

// ################## HELPER FUNCTIONS ##################

/// Removes spending entries that are outside the rolling window period.
/// Returns the total amount removed, which should be subtracted from
/// cached_total_spent.
///
/// # Arguments
///
/// * `spending_history` - The mutable history of spending transactions.
/// * `current_ledger` - The current ledger sequence.
/// * `period_ledgers` - The period in ledgers for the rolling window.
///
/// # Returns
///
/// The total amount of all removed entries.
fn cleanup_old_entries(
    spending_history: &mut Vec<SpendingEntry>,
    current_ledger: u32,
    period_ledgers: u32,
) -> i128 {
    let cutoff_ledger = current_ledger.saturating_sub(period_ledgers);
    let mut removed_total = 0i128;

    // Remove entries older than the cutoff ledger
    // We iterate from the front and remove old entries since they're at the
    // beginning
    while let Some(entry) = spending_history.get(0) {
        if entry.ledger_sequence < cutoff_ledger {
            removed_total += entry.amount;
            spending_history.pop_front();
        } else {
            break;
        }
    }

    removed_total
}
