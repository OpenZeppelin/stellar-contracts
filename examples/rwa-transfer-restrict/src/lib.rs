#![no_std]

use soroban_sdk::{contract, contractimpl, Address, String, Vec};
use stellar_tokens::rwa::compliance_modules::transfer_restrict::TransferRestrict;

#[contract]
pub struct TransferRestrictContract;

#[contractimpl(contracttrait)]
impl TransferRestrict for TransferRestrictContract {}
