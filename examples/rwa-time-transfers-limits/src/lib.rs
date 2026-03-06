#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::{
    compliance::ComplianceHook,
    compliance_modules::time_transfers_limits::{Limit, TimeTransfersLimits},
};

#[contract]
pub struct TimeTransfersLimitsContract;

#[contractimpl(contracttrait)]
impl TimeTransfersLimits for TimeTransfersLimitsContract {}
