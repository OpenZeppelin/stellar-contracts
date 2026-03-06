#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::{
    compliance::ComplianceHook,
    compliance_modules::initial_lockup_period::{InitialLockupPeriod, LockedTokens},
};

#[contract]
pub struct InitialLockupPeriodContract;

#[contractimpl(contracttrait)]
impl InitialLockupPeriod for InitialLockupPeriodContract {}
