use core::task::Context;

use cvlr::{
    cvlr_assert,
    nondet::{self, Nondet},
};
use cvlr_soroban::{nondet_address, nondet_vec};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, Vec};

use crate::policies::weighted_threshold::*;
