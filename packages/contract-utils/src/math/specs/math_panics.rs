use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::Nondet};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, I256};

use crate::math::{
    fixed_point::Rounding, i128_fixed_point::*, soroban_fixed_point::SorobanFixedPoint,
};

// todo: handle the muldiv function directly, need support for nondet_rounding.
// todo: overflow panics (not sure how)

#[rule]
// fixed_mul_floor panics if the denominator is 0
// status: verified
pub fn fixed_mul_floor_panics_if_zero_denominator(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    cvlr_assume!(z == 0);
    let _ = x.fixed_mul_floor(e, &y, &z);
    cvlr_assert!(false);
}

#[rule]
// fixed_mul_ceil panics if the denominator is 0
// status: verified
pub fn fixed_mul_ceil_panics_if_zero_denominator(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    cvlr_assume!(z == 0);
    let _ = x.fixed_mul_ceil(e, &y, &z);
    cvlr_assert!(false);
}

