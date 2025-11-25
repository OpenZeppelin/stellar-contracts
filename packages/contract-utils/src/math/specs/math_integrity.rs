use cvlr::{cvlr_assert,cvlr_assume,cvlr_satisfy};use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;
use soroban_sdk::{Env};
use crate::math::i128_fixed_point::*;
use crate::math::fixed_point::Rounding;
use crate::math::soroban_fixed_point::SorobanFixedPoint;

// todo: handle the muldiv function directly, need support for nondet_rounding.

#[rule]
// result is at most expected
// status: first assert verified, second violated spurious
// 
pub fn fixed_mul_floor_integrity(e: &Env) {
    let x = i128::nondet();
    clog!(x);
    let y = i128::nondet();
    clog!(y);
    let z = i128::nondet();
    clog!(z);
    let result = x.fixed_mul_floor(e, &y, &z);
    clog!(result);
    let expected_result = x * y / z;
    clog!(expected_result);
    let max_rounding_error: i128 = 1;
    clog!(max_rounding_error);
    let lower_bound = expected_result.checked_sub(max_rounding_error).unwrap();
    clog!(lower_bound);
    cvlr_assert!(result <= expected_result);
    cvlr_assert!(result >= lower_bound);
}

#[rule]
// result is at least expected
// status: first assert verified, second violated spurious
pub fn fixed_mul_ceil_integrity(e: &Env) {
    let x = i128::nondet();
    clog!(x);
    let y = i128::nondet();
    clog!(y);
    let z = i128::nondet();
    clog!(z);
    let result = x.fixed_mul_ceil(e, &y, &z);
    clog!(result);
    let expected_result = x * y / z;
    clog!(expected_result);
    let max_rounding_error = 1;
    clog!(max_rounding_error);
    let upper_bound = expected_result.checked_add(max_rounding_error).unwrap();
    clog!(upper_bound);
    cvlr_assert!(result >= expected_result);
    cvlr_assert!(result <= upper_bound);
}