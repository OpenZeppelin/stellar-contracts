#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::compliance_modules::country_restrict::CountryRestrict;

#[contract]
pub struct CountryRestrictContract;

#[contractimpl(contracttrait)]
impl CountryRestrict for CountryRestrictContract {}
