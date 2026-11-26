//! # Vesting Wallet
//!
//! A vesting contract holds SEP-41 tokens and releases them to a single
//! receiver following one fixed schedule (`start`, `cliff`, `end`). The
//! schedule is linear by default; a custom schedule is made by overriding
//! [`VestingCurve::vesting_curve`].
//!
//! ## Roles and where the tokens live
//!
//! - **Funder**: anyone who sends tokens to the vesting contract. A funder has
//!   no powers after the transfer.
//! - **Receiver**: the `Ownable` owner of the vesting contract
//!   ([`stellar_access::ownable`]). Released tokens always go to the current
//!   owner.
//! - **Vesting contract**: the contract built from this module.
//! - **Token contract**: the SEP-41 token. The vesting contract's balance is an
//!   entry in the token contract's storage, not in the vesting contract's.
//!
//! Only the vesting contract can authorize moving its own balance: it has no
//! `__check_auth`, so nobody can sign on its behalf, and its code moves tokens
//! only inside `release`, for the computed amount, to the owner.
//!
//! ## How a vesting contract works
//!
//! 1. **Deploy.** The constructor sets the receiver and the schedule. The
//!    schedule is made of absolute dates (`start`, `cliff`, `end`), fixed at
//!    deployment and never changed afterwards.
//! 2. **Fund.** Tokens are sent to the vesting contract's address with a plain
//!    transfer on the token contract, exactly like sending to any other
//!    address. For example, funding with XLM means sending XLM to the vesting
//!    contract's address. Anyone can fund, any number of times, and no function
//!    of the vesting contract is called.
//! 3. **Release.** As time passes, tokens vest according to the schedule.
//!    Anyone can call `release(token)` at any time, and the amount vested so
//!    far (minus what was already released) is sent to the receiver. No
//!    signature from the receiver is needed, which allows keepers to release on
//!    behalf of many vesting contracts.
//!
//! ### Funding does not start the schedule
//!
//! The schedule runs on its own dates, set at deployment, whether the
//! vesting contract holds any tokens or not. When the tokens arrive has no
//! effect on it.
//!
//! A transfer to the vesting contract only runs the token contract's code.
//! The vesting contract is not called: there is no hook and no notification,
//! so it cannot know when, or whether, it received tokens. All it can see is
//! its current balance, which it reads from the token contract whenever it
//! computes an amount.
//!
//! So a late (or forgotten and later sent) funding does not shift the
//! schedule: the tokens follow the original schedule, as if they had been sent
//! before `start`.
//!
//! ### Several tokens, one schedule
//!
//! Any SEP-41 token can be sent to the same vesting contract, and each one is
//! released separately with `release(token)`. The amount already released is
//! tracked per token, but every token follows the same single schedule.
//!
//! ### Several schedules: one vesting contract each
//!
//! A vesting contract holds exactly one schedule. To vest with a different
//! schedule (other dates or another curve), another vesting contract is
//! deployed with that schedule. To have a second, independent instance of
//! the same schedule (for another receiver, or a grant tracked apart),
//! another copy is deployed as well. Copies share the same uploaded Wasm, so
//! each deployment is cheap.
//!
//! ## Schedule
//!
//! The schedule is set once, in the constructor, through [`set_schedule`],
//! and never changes. For a token, `total_allocation` is the vesting
//! contract's current balance plus the amount already released. At a given
//! timestamp:
//!
//! - before `cliff`, nothing is vested;
//! - from `cliff` until `end`, the vested amount is
//!   [`VestingCurve::vesting_curve`] (linear by default);
//! - from `end` on, everything is vested.
//!
//! A `start` in the past is accepted; a vesting contract deployed after `end`
//! releases everything as soon as it is funded.
//!
//! Two setups give a simple timelock (everything at a single date):
//! - a future `start` with `duration = 0`;
//! - `cliff_duration == duration`.
//!
//! ## Custom schedules
//!
//! The curve lives in its own trait, [`VestingCurve`], which every vesting
//! contract implements next to [`VestingWallet`]. An empty
//! `impl VestingCurve for MyContract {}` keeps the linear curve; overriding
//! [`VestingCurve::vesting_curve`] in that impl replaces it.
//!
//! The override is only called for `cliff <= timestamp < end`, and its result
//! is checked by [`VestingWallet::vested_amount`]:
//! - a value below 0 or above `total_allocation` panics with
//!   [`VestingError::CurveOutOfBounds`];
//! - a curve that decreases below the amount already released makes `release`
//!   panic with [`VestingError::ClawbackOrDecreasingCurve`] until it catches
//!   up.
//!
//! An override must never decrease as `timestamp` grows. That rule is only
//! caught in the second case above, so it is the override author's
//! responsibility. Likewise, at a fixed timestamp, adding `d` tokens to the
//! allocation must raise the vested amount by at most `d`; otherwise a top-up
//! unlocks part of the original allocation early. The linear curve and the
//! monthly-steps example both satisfy this.
//!
//! See `examples/vesting-wallet/monthly-steps` for an override.
//!
//! ## Ownership
//!
//! - **Transferring ownership** moves every future release to the new owner,
//!   including tokens that have not vested yet. This allows the receiver to
//!   sell an unvested position.
//! - **Renouncing ownership** leaves no receiver: `release` then panics with
//!   [`VestingError::OwnerRenounced`], and every unreleased and future-vested
//!   token is locked forever. The library allows it; the examples override
//!   `renounce_ownership` to panic instead, which is the recommended setup.
//!
//! ## Payouts have no memo or muxed id
//!
//! `release` sends to the owner's plain `Address`, with no memo or muxed id.
//! The receiver should not be an exchange or custodian deposit address that
//! tells customers apart by memo: tokens sent there arrive on-chain but cannot
//! be credited to the right customer. This is not supported because the
//! receiver is the `Ownable` owner, which also has to sign for ownership
//! transfers, and a muxed id is not something that signs.
//!
//! ## Token behavior
//!
//! - Any SEP-41 token whose `transfer` takes exactly the sent amount from the
//!   sender is supported. This includes every Stellar Asset Contract (XLM and
//!   classic assets) and standard SEP-41 tokens. A fee deducted from the sent
//!   amount only reduces what the receiver gets; rebasing tokens follow the
//!   live balance.
//! - Tokens that take more than the sent amount from the sender (a fee charged
//!   on top) are not supported: every release then shrinks the allocation, and
//!   from `end` on `release` cannot pay the fee, so it reverts.
//! - A balance reduced outside `release` (an issuer clawback, a negative
//!   rebase) can bring the vested amount below the amount already released.
//!   Before `end`, `release` then panics with
//!   [`VestingError::ClawbackOrDecreasingCurve`] until the schedule catches up;
//!   from `end` on, the whole remaining balance is releasable. This is a stall,
//!   not a loss.
//! - A token that reports a negative balance is treated as having a balance of
//!   0.
//! - Token-side failures are not mapped to vesting errors: the release reverts
//!   unchanged and can be retried once the cause is fixed.
//!
//! ### Stellar Asset Contract tokens
//!
//! Before deploying a vesting contract for a Stellar asset, check the issuer
//! flags and the receiver's account:
//! - a G-account receiver needs a trustline for the asset, with a limit high
//!   enough for the released amounts;
//! - XLM sent to a G-account that does not exist yet creates the account if the
//!   payout is at least two base reserves; a smaller payout fails until the
//!   account exists;
//! - with `AUTH_REQUIRED`, funding the vesting contract fails until the issuer
//!   authorizes it;
//! - with `AUTH_REVOCABLE`, the issuer can deauthorize the vesting contract or
//!   the receiver and block releases;
//! - with clawback enabled, the issuer can reduce the vesting contract's
//!   balance (see above).
//!
//! ## Overflow bound
//!
//! `total_allocation` is computed with a checked addition and panics with
//! [`VestingError::AllocationOverflow`] above `i128::MAX` (about 1.7e38 base
//! units). Reaching it needs a token whose supply is close to `i128::MAX`, or
//! recycling released tokens back into the vesting contract at least 2^64
//! times at the SAC's per-transfer cap to a G-account (`i64::MAX`). From `end`
//! on, the releasable amount is the balance itself, so release resumes in full
//! even in that state. The amount stored as released then saturates at
//! `i128::MAX`: later deposits keep being released in full, but
//! [`VestingWallet::released`] reports `i128::MAX` forever and
//! [`VestingWallet::vested_amount`] panics with
//! [`VestingError::AllocationOverflow`] whenever the balance is not 0 (for a
//! timestamp at or after the cliff).
//!
//! ## Composition
//!
//! The vesting module calls `require_auth` nowhere; the only privileged
//! actions on a vesting contract are `Ownable`'s own. Making the vesting
//! contract upgradeable (or adding anything else that runs arbitrary code as
//! the vesting contract) gives that authority full control of the funds: an
//! upgrade can transfer every token at once.
//!
//! ## Example
//!
//! See `examples/vesting-wallet/linear` for a complete vesting contract.

mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, panic_with_error, Address, Env};
use stellar_access::ownable::get_owner;

pub use crate::vesting::storage::{
    get_released, get_schedule, linear_vesting_curve, release, set_schedule, token_balance,
    total_allocation, VestingSchedule, VestingStorageKey,
};

/// The vesting curve of a vesting contract: the shape of its schedule between
/// `cliff` and `end`.
///
/// Called by [`VestingWallet::vested_amount`] to compute the amount vested at a
/// given timestamp. The default is linear, but a vesting contract can override
/// it to implement a custom curve. See `examples/vesting-wallet/monthly-steps`
/// for an example of a custom curve.
///
/// `impl VestingCurve for MyContract {}` keeps the linear curve; overriding
/// [`VestingCurve::vesting_curve`] in that impl gives a custom one.
pub trait VestingCurve {
    /// Returns the amount vested at `timestamp` out of `total_allocation`.
    /// Default: linear, see [`linear_vesting_curve`].
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
    /// Rules for an override:
    /// * It is only called with `cliff <= timestamp < end`, so `duration > 0`.
    /// * It must return a value in `[0, total_allocation]`; otherwise
    ///   [`VestingWallet::vested_amount`] panics with
    ///   [`VestingError::CurveOutOfBounds`].
    /// * It must never decrease as `timestamp` grows. A decrease below the
    ///   amount already released makes `release` panic with
    ///   [`VestingError::ClawbackOrDecreasingCurve`]; a smaller decrease is not
    ///   detected.
    /// * At a fixed `timestamp`, adding `d` to `total_allocation` must raise
    ///   the result by at most `d`. Otherwise a top-up unlocks part of the
    ///   original allocation early.
    fn vesting_curve(
        e: &Env,
        schedule: &VestingSchedule,
        total_allocation: i128,
        timestamp: u64,
    ) -> i128 {
        storage::linear_vesting_curve(e, schedule, total_allocation, timestamp)
    }
}

/// Vesting of SEP-41 tokens on a single schedule, released to the `Ownable`
/// owner.
///
/// Every method has a default body and applies the vesting rules; none is
/// meant to be overridden. A vesting contract also implements
/// [`VestingCurve`], which sets the shape of the schedule.
#[contracttrait]
pub trait VestingWallet: VestingCurve {
    /// Returns the timestamp (in seconds) at which vesting starts.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`get_schedule`] errors.
    fn start(e: &Env) -> u64 {
        storage::get_schedule(e).start
    }

    /// Returns the length of the vesting period in seconds.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`get_schedule`] errors.
    fn duration(e: &Env) -> u64 {
        storage::get_schedule(e).duration
    }

    /// Returns the timestamp (in seconds) before which nothing is vested.
    /// Equal to `start` when there is no cliff.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`get_schedule`] errors.
    fn cliff(e: &Env) -> u64 {
        storage::get_schedule(e).cliff
    }

    /// Returns the timestamp (in seconds) from which everything is vested.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`get_schedule`] errors.
    fn end(e: &Env) -> u64 {
        storage::get_schedule(e).end
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
    /// tokens, but the value returned here is wrong: it shows `i128::MAX`
    /// forever, below the true cumulative amount.
    fn released(e: &Env, token: Address) -> i128 {
        storage::get_released(e, &token)
    }

    /// Returns the total amount of `token` vested at `timestamp`, already
    /// released or not.
    ///
    /// - before `cliff`: `0` (the balance is not read);
    /// - from `end` on: the total allocation (`balance + released`);
    /// - otherwise: [`VestingCurve::vesting_curve`], checked to be within `[0,
    ///   total allocation]`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The address of the token contract.
    /// * `timestamp` - The timestamp (in seconds) to compute the amount for.
    ///
    /// # Errors
    ///
    /// * [`VestingError::CurveOutOfBounds`] - When the curve returns a value
    ///   below 0 or above the total allocation.
    /// * refer to [`get_schedule`] errors.
    /// * refer to [`total_allocation`] errors. This includes the state after
    ///   the overflow scenario described in the module docs, where this
    ///   function panics whenever the balance is not 0 (for a timestamp at or
    ///   after the cliff).
    fn vested_amount(e: &Env, token: Address, timestamp: u64) -> i128 {
        let schedule = storage::get_schedule(e);

        // Checked before reading the balance: nothing is vested before the
        // cliff, so there is no need to call the token contract.
        if timestamp < schedule.cliff {
            return 0;
        }

        let total = storage::total_allocation(e, &token);

        if timestamp >= schedule.end {
            return total;
        }

        // Here `cliff <= timestamp < end`, so `duration > 0` inside the curve.
        let vested = Self::vesting_curve(e, &schedule, total, timestamp);

        // A custom curve could return anything; keep it within the allocation.
        if vested < 0 || vested > total {
            panic_with_error!(e, VestingError::CurveOutOfBounds);
        }

        vested
    }

    /// Returns the amount of `token` that [`VestingWallet::release`] would
    /// send now.
    ///
    /// - from `end` on: the vesting contract's whole balance;
    /// - otherwise: the vested amount minus the amount already released.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The address of the token contract.
    ///
    /// # Errors
    ///
    /// * [`VestingError::ClawbackOrDecreasingCurve`] - Before `end`, when the
    ///   vested amount is below the amount already released.
    /// * refer to [`VestingWallet::vested_amount`] errors (before `end` only).
    /// * refer to [`get_schedule`] errors.
    fn releasable(e: &Env, token: Address) -> i128 {
        let now = e.ledger().timestamp();

        // From `end` on, everything left is releasable. The
        // `balance + released` sum is skipped on purpose: in the (practically
        // unreachable) case where it overflows, tokens still get released.
        if now >= storage::get_schedule(e).end {
            return storage::token_balance(e, &token);
        }

        let vested = Self::vested_amount(e, token.clone(), now);
        let released = storage::get_released(e, &token);

        // NOTE: can't overflow because both values are in `[0, i128::MAX]`.
        let releasable = vested - released;

        // Panic rather than return 0, so a clawback or a decreasing curve is
        // reported instead of silently stalling.
        if releasable < 0 {
            panic_with_error!(e, VestingError::ClawbackOrDecreasingCurve);
        }

        releasable
    }

    /// Sends the releasable amount of `token` to the current owner and returns
    /// it. Callable by anyone.
    ///
    /// Returns `0` without a transfer, an event, or a change to the released
    /// amount when nothing is releasable. Reading an existing released-amount
    /// entry still extends its TTL.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The address of the token contract.
    ///
    /// # Errors
    ///
    /// * refer to [`VestingWallet::releasable`] errors.
    /// * [`VestingError::OwnerRenounced`] - When there is no owner, even if
    ///   nothing is releasable.
    ///
    /// # Events
    ///
    /// * topics - `["released", token: Address, to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// No authorization is required: the tokens can only go to the current
    /// owner. Token-side failures (no trustline, deauthorized holder, ...)
    /// revert the call unchanged.
    ///
    /// A contract that picks its receiver differently (a fixed address, a
    /// role) can override this function and call [`release`] with its own
    /// receiver; the storage functions do not depend on `Ownable`.
    fn release(e: &Env, token: Address) -> i128 {
        let amount = Self::releasable(e, token.clone());
        // Checked even when `amount` is 0, so a renounced contract always
        // reports why nothing can be released.
        let Some(owner) = get_owner(e) else {
            panic_with_error!(e, VestingError::OwnerRenounced);
        };
        storage::release(e, &token, &owner, amount)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VestingError {
    /// The vesting schedule has not been set.
    ScheduleNotSet = 7000,
    /// The vesting schedule has already been set.
    ScheduleAlreadySet = 7001,
    /// The cliff duration is longer than the vesting duration.
    InvalidCliffDuration = 7002,
    /// `start + duration` does not fit in a `u64` timestamp.
    EndOverflow = 7003,
    /// Ownership has been renounced, so there is no receiver for released
    /// tokens.
    OwnerRenounced = 7004,
    /// A custom `vesting_curve` returned a value below 0 or above the total
    /// allocation.
    CurveOutOfBounds = 7005,
    /// The vested amount is below the amount already released. Either the
    /// vesting contract's balance was reduced outside `release` (an issuer
    /// clawback, a negatively rebasing token, or a misbehaving token), or a
    /// custom `vesting_curve` decreased over time.
    ClawbackOrDecreasingCurve = 7006,
    /// The token balance plus the amount already released does not fit in an
    /// `i128`.
    AllocationOverflow = 7007,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const RELEASED_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const RELEASED_TTL_THRESHOLD: u32 = RELEASED_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when vested tokens are released to the owner.
///
/// `token` is chosen by the caller of `release`, so consumers must filter on
/// it: any contract address can appear here.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Released {
    #[topic]
    pub token: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

/// Emits an event when vested tokens are released to the owner.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The address of the token contract.
/// * `to` - The owner the tokens are sent to.
/// * `amount` - The amount released.
pub fn emit_released(e: &Env, token: &Address, to: &Address, amount: i128) {
    Released { token: token.clone(), to: to.clone(), amount }.publish(e);
}

/// Event emitted when the vesting schedule is set.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingScheduleSet {
    pub start: u64,
    pub duration: u64,
    pub cliff: u64,
    pub end: u64,
}

/// Emits an event when the vesting schedule is set.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `schedule` - The stored vesting schedule.
pub fn emit_vesting_schedule_set(e: &Env, schedule: &VestingSchedule) {
    VestingScheduleSet {
        start: schedule.start,
        duration: schedule.duration,
        cliff: schedule.cliff,
        end: schedule.end,
    }
    .publish(e);
}
