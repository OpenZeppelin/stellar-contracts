use soroban_sdk::{contracttype, panic_with_error, token::TokenClient, Address, Env};
use stellar_access::ownable::get_owner;
use stellar_contract_utils::math::i128_fixed_point::mul_div_floor;

use crate::vesting::{
    emit_released, emit_vesting_schedule_set, VestingError, RELEASED_EXTEND_AMOUNT,
    RELEASED_TTL_THRESHOLD,
};

/// The vesting schedule. Set once by [`set_schedule`], never changed.
///
/// Every stored schedule satisfies `start <= cliff <= end` and
/// `end - start == duration`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingSchedule {
    /// Ledger timestamp (seconds) at which vesting starts.
    pub start: u64,
    /// Length of the vesting period in seconds.
    pub duration: u64,
    /// Ledger timestamp (seconds) before which nothing is vested. Equal to
    /// `start` when there is no cliff.
    pub cliff: u64,
    /// Ledger timestamp (seconds) from which everything is vested.
    pub end: u64,
}

/// Storage keys for the vesting module.
#[contracttype]
pub enum VestingStorageKey {
    /// The [`VestingSchedule`]. Instance storage.
    Schedule,
    /// Amount of the token already released (`i128`). Persistent storage.
    Released(Address),
}

// ################## QUERY STATE ##################

/// Returns the vesting schedule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`VestingError::ScheduleNotSet`] - When the schedule has not been set.
pub fn get_schedule(e: &Env) -> VestingSchedule {
    e.storage()
        .instance()
        .get(&VestingStorageKey::Schedule)
        .unwrap_or_else(|| panic_with_error!(e, VestingError::ScheduleNotSet))
}

/// Returns the amount of `token` already released. Defaults to `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract.
///
/// # Notes
///
/// In the overflow scenario described in the module docs (practically
/// unreachable), the vesting contract keeps working and keeps releasing
/// tokens, but the value returned here is wrong: it shows `i128::MAX` forever,
/// below the true cumulative amount.
pub fn get_released(e: &Env, token: &Address) -> i128 {
    let key = VestingStorageKey::Released(token.clone());
    // A missing entry reads as 0 and is not created by the read.
    if let Some(released) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, RELEASED_TTL_THRESHOLD, RELEASED_EXTEND_AMOUNT);
        released
    } else {
        0
    }
}

/// Returns the vesting contract's balance of `token`, as reported by the token
/// contract. A negative balance (only possible with a misbehaving token) is
/// returned as `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract.
pub fn token_balance(e: &Env, token: &Address) -> i128 {
    // Only a misbehaving token can report a negative balance. Treating it as
    // 0 keeps it from reverting a keeper's batch of releases.
    TokenClient::new(e, token).balance(&e.current_contract_address()).max(0)
}

/// Returns the total allocation of `token`: the vesting contract's balance
/// plus the amount already released.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract.
///
/// # Errors
///
/// * [`VestingError::AllocationOverflow`] - When the sum does not fit in an
///   `i128`.
pub fn total_allocation(e: &Env, token: &Address) -> i128 {
    token_balance(e, token)
        .checked_add(get_released(e, token))
        .unwrap_or_else(|| panic_with_error!(e, VestingError::AllocationOverflow))
}

/// The default linear vesting curve:
/// `floor(total_allocation * (timestamp - start) / duration)`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `schedule` - The vesting schedule.
/// * `total_allocation` - The vesting contract's balance plus the amount
///   already released.
/// * `timestamp` - The timestamp (in seconds) to compute the amount for.
///
/// # Notes
///
/// Meant to be called only with `cliff <= timestamp < end`, as
/// [`crate::vesting::VestingWallet::vested_amount`] does. In that range
/// `duration > 0`, and the result is in `[0, total_allocation)` when
/// `total_allocation > 0`. Flooring means rounding never favors the
/// receiver; the remainder is paid at `end`.
pub fn linear_vesting_curve(
    e: &Env,
    schedule: &VestingSchedule,
    total_allocation: i128,
    timestamp: u64,
) -> i128 {
    // Through `vested_amount`, `timestamp >= cliff >= start`. Saturating only
    // matters for a direct call with a timestamp before `start`, which gets 0.
    let elapsed = timestamp.saturating_sub(schedule.start);
    // `u64 -> i128` casts are lossless. The result is at most
    // `total_allocation`, so `mul_div_floor` cannot overflow.
    mul_div_floor(e, &total_allocation, &(elapsed as i128), &(schedule.duration as i128))
}

// ################## CHANGE STATE ##################

/// Sets the vesting schedule. Stores the absolute `cliff = start +
/// cliff_duration` and `end = start + duration`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `start` - Ledger timestamp (seconds) at which vesting starts. A past
///   timestamp is accepted.
/// * `duration` - Length of the vesting period in seconds. `0` is accepted
///   (everything vests at `start`).
/// * `cliff_duration` - Seconds after `start` before which nothing is vested.
///   `0` means no cliff.
///
/// # Errors
///
/// * [`VestingError::ScheduleAlreadySet`] - When the schedule is already set.
/// * [`VestingError::InvalidCliffDuration`] - When `cliff_duration > duration`.
/// * [`VestingError::EndOverflow`] - When `start + duration` does not fit in a
///   `u64`.
///
/// # Events
///
/// * topics - `["vesting_schedule_set"]`
/// * data - `[start: u64, duration: u64, cliff: u64, end: u64]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks. It can only
/// succeed once, and is meant to be called only in the constructor.
pub fn set_schedule(e: &Env, start: u64, duration: u64, cliff_duration: u64) {
    if e.storage().instance().has(&VestingStorageKey::Schedule) {
        panic_with_error!(e, VestingError::ScheduleAlreadySet);
    }

    if cliff_duration > duration {
        panic_with_error!(e, VestingError::InvalidCliffDuration);
    }

    let end = start
        .checked_add(duration)
        .unwrap_or_else(|| panic_with_error!(e, VestingError::EndOverflow));

    // NOTE: can't overflow because `cliff_duration <= duration` and
    // `start + duration` already fits.
    let cliff = start + cliff_duration;

    let schedule = VestingSchedule { start, duration, cliff, end };
    e.storage().instance().set(&VestingStorageKey::Schedule, &schedule);

    emit_vesting_schedule_set(e, &schedule);
}

/// Sends `amount` of `token` from the vesting contract to the current owner,
/// records it as released, and returns it. Returns `0` without a transfer, an
/// event, or a change to the released amount when `amount` is `0` (reading an
/// existing released-amount entry still extends its TTL).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract.
/// * `amount` - The amount to release.
///
/// # Errors
///
/// * [`VestingError::OwnerRenounced`] - When there is no owner, even if
///   `amount` is `0`.
///
/// # Events
///
/// * topics - `["released", token: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// The amount released is added with saturation. In the overflow scenario
/// described in the module docs (practically unreachable), the vesting
/// contract keeps working and keeps releasing tokens, but the stored released
/// amount is wrong: it shows `i128::MAX` forever.
///
/// # Security Warning
///
/// **IMPORTANT**: This function lacks authorization checks and trusts
/// `amount`: it applies none of the vesting rules. Passing anything other than
/// the result of [`crate::vesting::VestingWallet::releasable`], computed in the
/// same invocation, can pay out unvested tokens.
pub fn release(e: &Env, token: &Address, amount: i128) -> i128 {
    // Checked even when `amount` is 0, so a renounced contract always
    // reports why nothing can be released.
    let Some(owner) = get_owner(e) else {
        panic_with_error!(e, VestingError::OwnerRenounced);
    };

    if amount == 0 {
        return 0;
    }

    // Before `end` the sum equals the vested amount, which already fits.
    // After `end` it can only saturate in the overflow scenario described in
    // the module docs, where a checked add would lock the remaining tokens.
    let released = get_released(e, token).saturating_add(amount);
    e.storage().persistent().set(&VestingStorageKey::Released(token.clone()), &released);

    // State is updated before the external call. If the transfer fails, the
    // whole invocation reverts, including the write and the event.
    emit_released(e, token, &owner, amount);

    // No signature is needed: the vesting contract is the direct caller, and
    // this is the only place it moves its own tokens.
    TokenClient::new(e, token).transfer(&e.current_contract_address(), &owner, &amount);

    amount
}
