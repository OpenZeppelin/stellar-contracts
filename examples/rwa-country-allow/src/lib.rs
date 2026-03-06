#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::compliance_modules::country_allow::CountryAllow;

#[contract]
pub struct CountryAllowContract;

#[contractimpl(contracttrait)]
impl CountryAllow for CountryAllowContract {}
