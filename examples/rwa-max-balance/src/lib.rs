#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::compliance::ComplianceHook;
use stellar_tokens::rwa::compliance_modules::max_balance::MaxBalance;

#[contract]
pub struct MaxBalanceContract;

#[contractimpl(contracttrait)]
impl MaxBalance for MaxBalanceContract {}
