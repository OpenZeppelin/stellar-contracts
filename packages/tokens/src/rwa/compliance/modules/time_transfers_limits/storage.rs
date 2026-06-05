use soroban_sdk::{contracttype, panic_with_error, Address, Env, Vec};

use crate::rwa::compliance::modules::{
    storage::{add_i128_or_panic, get_irs_client, require_non_negative_amount},
    time_transfers_limits::{
        emit_time_transfer_limit_removed, emit_time_transfer_limit_set, MAX_LIMITS,
    },
    ComplianceModuleError, MODULE_EXTEND_AMOUNT, MODULE_TTL_THRESHOLD,
};

/// A single time-window limit configured for a token: at most `limit_value`
/// tokens may be sent within a window lasting `limit_duration` ledgers. A
/// window opens with the first transfer after the previous one elapsed;
/// per-identity consumption against the cap is tracked by
/// [`TransferCounter`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferLimit {
    pub limit_duration: u32,
    pub limit_value: i128,
}

/// The cumulative volume one identity has sent within its currently active
/// window: `value` accumulates against the matching [`TransferLimit`]'s cap
/// until the ledger sequence reaches `deadline` (the moment the window
/// ends), after which the next transfer restarts the counter for a fresh
/// window.
///
/// An identity will/may have multiple active counters at once if
/// multiple limits are configured for the token, one for each distinct
/// window duration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferCounter {
    pub value: i128,
    pub deadline: u32,
}

/// Storage key fields for a per-(token, identity, window) counter entry.
#[contracttype]
#[derive(Clone)]
pub struct TransferCounterKey {
    pub token: Address,
    pub identity: Address,
    pub limit_duration: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum TimeTransfersLimitsStorageKey {
    /// Per-token list of configured time-window limits.
    Limits(Address),
    /// Per-(token, identity, window) cumulative transfer counter.
    Counter(TransferCounterKey),
}

// ################## QUERY STATE ##################

/// Returns the time-window limits configured for `token`. Returns an empty
/// vector when no limits have been configured, in which case transfers are
/// not restricted by this module.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
pub fn get_time_transfer_limits(e: &Env, token: &Address) -> Vec<TransferLimit> {
    let key = TimeTransfersLimitsStorageKey::Limits(token.clone());
    if let Some(value) = e.storage().persistent().get::<_, Vec<TransferLimit>>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        Vec::new(e)
    }
}

/// Returns the transfer counter tracked for `identity` within the
/// `limit_duration` window under `token`. Returns a zeroed counter when no
/// entry exists.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `identity` - The identity (on-chain ID) address.
/// * `limit_duration` - The window duration in ledgers.
pub fn get_transfer_counter(
    e: &Env,
    token: &Address,
    identity: &Address,
    limit_duration: u32,
) -> TransferCounter {
    let key = TimeTransfersLimitsStorageKey::Counter(TransferCounterKey {
        token: token.clone(),
        identity: identity.clone(),
        limit_duration,
    });
    if let Some(value) = e.storage().persistent().get::<_, TransferCounter>(&key) {
        e.storage().persistent().extend_ttl(&key, MODULE_TTL_THRESHOLD, MODULE_EXTEND_AMOUNT);
        value
    } else {
        TransferCounter { value: 0, deadline: 0 }
    }
}

/// Returns `true` when sending `amount` satisfies every configured
/// time-window limit for the sender's identity: the amount alone must not
/// exceed any window's cap, and within each unexpired window the running
/// counter plus `amount` must stay at or below the cap.
///
/// Only the sender side is consulted; the recipient is intentionally
/// ignored. The sender's identity is resolved through the token's IRS only
/// when at least one limit is configured.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `_to` - The recipient address.
/// * `amount` - The transfer amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathOverflow`] - When the projected counter
///   addition overflows.
/// * refer to [`get_irs_client`] errors.
pub fn can_transfer(e: &Env, from: &Address, _to: &Address, amount: i128, token: &Address) -> bool {
    require_non_negative_amount(e, amount);

    let limits = get_time_transfer_limits(e, token);
    if limits.is_empty() {
        return true;
    }

    let identity = get_irs_client(e, token).stored_identity(from);
    let now = e.ledger().sequence();
    for limit in limits.iter() {
        if amount > limit.limit_value {
            return false;
        }
        let counter = get_transfer_counter(e, token, &identity, limit.limit_duration);

        // if the window is still open && the new counter value would exceed the limit
        if counter.deadline > now && add_i128_or_panic(e, counter.value, amount) > limit.limit_value
        {
            return false;
        }
    }
    true
}

// ################## CHANGE STATE ##################

/// Adds or updates a time-window limit for `token`. A limit whose
/// `limit_duration` matches an existing entry replaces it; otherwise the limit
/// is appended.
///
/// No duplicate `limit_duration` values are allowed. If a limit with the same
/// `limit_duration` already exists, it is replaced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit` - The window duration (in ledgers) and the volume cap.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `limit.limit_value` is
///   negative.
/// * [`ComplianceModuleError::LimitBoundExceeded`] - When appending a new
///   window would exceed the max limit bound.
///
/// # Events
///
/// * topics - `["time_transfer_limit_set", token: Address]`
/// * data - `[limit_duration: u32, limit_value: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn set_time_transfer_limit(e: &Env, token: &Address, limit: &TransferLimit) {
    require_non_negative_amount(e, limit.limit_value);

    let mut limits = get_time_transfer_limits(e, token);
    match limits.iter().position(|l| l.limit_duration == limit.limit_duration) {
        Some(index) => limits.set(index as u32, limit.clone()),
        None if limits.len() >= MAX_LIMITS => {
            panic_with_error!(e, ComplianceModuleError::LimitBoundExceeded)
        }
        None => limits.push_back(limit.clone()),
    }

    e.storage().persistent().set(&TimeTransfersLimitsStorageKey::Limits(token.clone()), &limits);
    emit_time_transfer_limit_set(e, token, limit.limit_duration, limit.limit_value);
}

/// Adds or updates multiple time-window limits in a single call.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limits` - The limits to add or update.
///
/// # Errors
///
/// * refer to [`set_time_transfer_limit`] errors.
///
/// # Events
///
/// For each limit:
/// * topics - `["time_transfer_limit_set", token: Address]`
/// * data - `[limit_duration: u32, limit_value: i128]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_set_time_transfer_limit(e: &Env, token: &Address, limits: &Vec<TransferLimit>) {
    for limit in limits.iter() {
        set_time_transfer_limit(e, token, &limit);
    }
}

/// Removes the time-window limit with duration `limit_duration` for `token`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit_duration` - The window duration (in ledgers) to remove.
///
/// # Errors
///
/// * [`ComplianceModuleError::LimitNotFound`] - When no limit exists for
///   `limit_duration`.
///
/// # Events
///
/// * topics - `["time_transfer_limit_removed", token: Address]`
/// * data - `[limit_duration: u32]`
///
/// # Notes
///
/// Counters recorded for the removed window are not erased. If the same
/// window duration is configured again later, an unexpired counter resumes
/// counting where it left off.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn remove_time_transfer_limit(e: &Env, token: &Address, limit_duration: u32) {
    let mut limits = get_time_transfer_limits(e, token);
    let index = limits
        .iter()
        .position(|l| l.limit_duration == limit_duration)
        .unwrap_or_else(|| panic_with_error!(e, ComplianceModuleError::LimitNotFound));
    limits.remove(index as u32);

    e.storage().persistent().set(&TimeTransfersLimitsStorageKey::Limits(token.clone()), &limits);
    emit_time_transfer_limit_removed(e, token, limit_duration);
}

/// Removes multiple time-window limits in a single call.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token address.
/// * `limit_durations` - The window durations (in ledgers) to remove.
///
/// # Errors
///
/// * refer to [`remove_time_transfer_limit`] errors.
///
/// # Events
///
/// For each removed limit:
/// * topics - `["time_transfer_limit_removed", token: Address]`
/// * data - `[limit_duration: u32]`
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn batch_remove_time_transfer_limit(e: &Env, token: &Address, limit_durations: &Vec<u32>) {
    for limit_duration in limit_durations.iter() {
        remove_time_transfer_limit(e, token, limit_duration);
    }
}

/// Records a transfer of `amount` against every configured time-window
/// limit: the sender's identity is resolved through the token's IRS, each
/// elapsed counter restarts for a fresh window, and the running counters
/// are incremented. Panics when an increment would push a counter past its
/// window's cap.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender address.
/// * `_to` - The recipient address.
/// * `amount` - The transferred amount.
/// * `token` - The token address.
///
/// # Errors
///
/// * [`ComplianceModuleError::InvalidAmount`] - When `amount` is negative.
/// * [`ComplianceModuleError::MathOverflow`] - When a counter addition
///   overflows.
/// * [`ComplianceModuleError::TransferLimitExceeded`] - When the new counter
///   value would exceed a configured window's cap.
/// * refer to [`get_irs_client`] errors.
///
/// # Security Warning
///
/// This helper performs no authorization checks.
pub fn on_transfer(e: &Env, from: &Address, _to: &Address, amount: i128, token: &Address) {
    require_non_negative_amount(e, amount);

    let limits = get_time_transfer_limits(e, token);
    if limits.is_empty() {
        return;
    }

    let identity = get_irs_client(e, token).stored_identity(from);
    let now = e.ledger().sequence();
    for limit in limits.iter() {
        let mut counter = get_transfer_counter(e, token, &identity, limit.limit_duration);
        if counter.deadline <= now {
            // The previous window elapsed: restart counting.
            counter =
                TransferCounter { value: 0, deadline: now.saturating_add(limit.limit_duration) };
        }
        counter.value = add_i128_or_panic(e, counter.value, amount);
        if counter.value > limit.limit_value {
            panic_with_error!(e, ComplianceModuleError::TransferLimitExceeded);
        }

        // Set the updated counter back in storage.
        let key = TimeTransfersLimitsStorageKey::Counter(TransferCounterKey {
            token: token.clone(),
            identity: identity.clone(),
            limit_duration: limit.limit_duration,
        });
        e.storage().persistent().set(&key, &counter);
    }
}
