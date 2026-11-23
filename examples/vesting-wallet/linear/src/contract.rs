//! Vesting Wallet Example Contract (linear schedule).
//!
//! Holds any SEP-41 tokens sent to it and releases them to the owner. Nothing
//! is available before the cliff; after that, tokens unlock linearly until
//! `end`. Setup is a deployment followed by a plain token transfer to this
//! contract.
//!
//! `renounce_ownership` is disabled: without an owner, `release` has nowhere
//! to send tokens.

use soroban_sdk::{contract, contracterror, contractimpl, panic_with_error, Address, Env};
use stellar_access::ownable::{set_owner, Ownable};
// `VestingSchedule` must be in scope: `#[contractimpl(contracttrait)]`
// generates the `vesting_curve` contract function here, and it takes a
// `VestingSchedule` parameter.
use stellar_finance::vesting::{set_schedule, VestingSchedule, VestingWallet};

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
impl VestingWallet for VestingWalletContract {}

#[contractimpl(contracttrait)]
impl Ownable for VestingWalletContract {
    /// Disabled: with no owner, `release` has nowhere to send tokens, so every
    /// unreleased and future-vested token would be locked forever.
    fn renounce_ownership(e: &Env) {
        panic_with_error!(e, ExampleError::RenounceDisabled);
    }
}
