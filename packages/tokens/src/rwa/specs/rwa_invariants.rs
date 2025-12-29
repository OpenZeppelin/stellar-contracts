use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::*};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::rwa::RWA;
use crate::fungible::ContractOverrides;
use crate::fungible::FungibleToken;

// relevant functions:
// forced_transfer, mint, burn, recover_balance,
// set_address_frozen, freeze_partial_tokens, unfreeze_partial_tokens
// set_compliance, set_identity_verifier
// approve, transfer, transfer_from

// get_frozen_tokens <= balance
// get_frozen_tokens >= 0
// balance > 0 -> identity verifier passed