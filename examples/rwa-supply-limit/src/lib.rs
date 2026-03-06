#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::compliance::ComplianceHook;
use stellar_tokens::rwa::compliance_modules::supply_limit::SupplyLimit;

#[contract]
pub struct SupplyLimitContract;

#[contractimpl(contracttrait)]
impl SupplyLimit for SupplyLimitContract {}
