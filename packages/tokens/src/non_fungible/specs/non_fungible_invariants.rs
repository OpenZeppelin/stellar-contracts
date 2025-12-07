use cvlr::{cvlr_satisfy, nondet::*, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};
use cvlr::clog;

use crate::non_fungible::Base;

// invariant: token_owner exists
// invariant: token_owner -> balance >= 1 (can't do iff)