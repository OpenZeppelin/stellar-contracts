use cvlr::{cvlr_assert,cvlr_assume,cvlr_satisfy};use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;
use cvlr::clog;
use soroban_sdk::{Env};
use crate::math::i128_fixed_point::*;
use crate::math::fixed_point::Rounding;
use crate::math::soroban_fixed_point::SorobanFixedPoint;

// todo: handle the muldiv function directly, need support for nondet_rounding.

pub fn abs(x: i128) -> i128 {
    if x < 0 {
        -x
    } else {
        x
    }
}

#[rule]
// result is at most expected
// status: first assert verified
// second assert violated - seems spurious
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

    let max_rounding_error = abs(1/z);
    clog!(max_rounding_error);
    clog!(expected_result - max_rounding_error);
    
    cvlr_assert!(result <= expected_result);
    cvlr_assert!(result >= expected_result - max_rounding_error);
}

#[rule]
// result is at least expected
// status: first assert verified
// second assert violated - seems spurious
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
    let max_rounding_error = abs(1/z);
    clog!(max_rounding_error);
    clog!(expected_result + max_rounding_error);
    cvlr_assert!(result >= expected_result);
    cvlr_assert!(result <= expected_result + max_rounding_error);
}