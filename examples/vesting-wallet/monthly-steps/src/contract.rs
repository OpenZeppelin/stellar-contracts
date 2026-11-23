//! Vesting Wallet Example Contract (monthly steps).
//!
//! Same as the linear example, but tokens unlock in monthly pieces instead of
//! continuously. It shows how a custom schedule is made by overriding a single
//! method, `vesting_curve`; the library still applies the cliff, the end, and
//! the bounds checks around it.
//!
//! A month is a fixed 30 days (2,592,000 seconds), not a calendar month.

use soroban_sdk::{contract, contracterror, contractimpl, panic_with_error, Address, Env};
use stellar_access::ownable::{set_owner, Ownable};
use stellar_contract_utils::math::i128_fixed_point::mul_div_floor;
use stellar_finance::vesting::{set_schedule, VestingSchedule, VestingWallet};

/// A fixed 30-day month, in seconds.
pub const MONTH: u64 = 30 * 24 * 60 * 60;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleError {
    /// Renouncing would leave vested tokens with no receiver.
    RenounceDisabled = 1,
}

#[contract]
pub struct VestingWalletContract;

#[contractimpl]
impl VestingWalletContract {
    pub fn __constructor(
        e: &Env,
        receiver: Address,
        start: u64,
        duration: u64,
        cliff_duration: u64,
    ) {
        set_owner(e, &receiver);
        set_schedule(e, start, duration, cliff_duration);
    }
}

#[contractimpl(contracttrait)]
impl VestingWallet for VestingWalletContract {
    /// Vests `total_allocation` in monthly pieces:
    /// `total_allocation * months_passed / total_months`, floored.
    ///
    /// The two values are the two halves of a fraction, "pieces unlocked so
    /// far / pieces in total", the monthly counterpart of the linear curve's
    /// `elapsed / duration`:
    /// - `months_passed` counts full months since `start` (rounded down), so a
    ///   piece unlocks only once its month has fully passed;
    /// - `total_months` counts the pieces (rounded up), so a partial last month
    ///   is a piece of its own, paid at `end`.
    ///
    /// The library only calls this with `cliff <= timestamp < end`, so
    /// `months_passed < total_months` and the result stays below
    /// `total_allocation`. A schedule shorter than one month has one piece:
    /// nothing before `end`, everything at `end`.
    fn vesting_curve(
        e: &Env,
        schedule: VestingSchedule,
        total_allocation: i128,
        timestamp: u64,
    ) -> i128 {
        let months_passed = timestamp.saturating_sub(schedule.start) / MONTH;
        let total_months = schedule.duration.div_ceil(MONTH);
        mul_div_floor(e, &total_allocation, &(months_passed as i128), &(total_months as i128))
    }
}

#[contractimpl(contracttrait)]
impl Ownable for VestingWalletContract {
    /// Disabled: with no owner, `release` has nowhere to send tokens, so every
    /// unreleased and future-vested token would be locked forever.
    fn renounce_ownership(e: &Env) {
        panic_with_error!(e, ExampleError::RenounceDisabled);
    }
}
