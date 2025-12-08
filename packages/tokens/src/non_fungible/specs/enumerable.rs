use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;
use crate::non_fungible::Base;


// ################## INVARIANTS ##################

// invariant: index < total_supply <-> get_token_id != none
// invariant: index < balance <-> get_owner_token_id != none
// invariant total_supply >= balance

// invariants should be checked for transfer, transfer_from, mint, sequential_mint, burn and burn_from (approves are trivial)
// need wrapper of get_token_id and get_owner_token_id to have them return None instead of panicking.

// related somewhat to what we had in access_control
