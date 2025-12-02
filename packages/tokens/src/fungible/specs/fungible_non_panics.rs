use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr_soroban::{nondet_address, is_auth};
use cvlr::clog;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::fungible::FungibleToken;
use crate::fungible::Base;

// #[rule]
// requires
// 
// status: 
pub fn transfer_non_panic(e: Env) {
}

// transfer_from non_panic

// approve non_panic