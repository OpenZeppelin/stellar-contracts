use cvlr::{clog, cvlr_assert, cvlr_assume, cvlr_satisfy, nondet::Nondet};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::math::{
    fixed_point::Rounding, i128_fixed_point::*, soroban_fixed_point::SorobanFixedPoint,
};

// todo: handle the muldiv function directly, need support for nondet_rounding.

#[rule]
// result is at most expected
// status: verified https://prover.certora.com/output/33158/cee6ba53f3f14674851955c8aec9a0c1
// sanity: https://prover.certora.com/output/33158/465304f6d768450d8660b8c25290c0ee
// NOTE: see the usage of certora feature in `i128_fixed_point.rs` that skips the 256 bit attempt due to 64 bit assumptions here
pub fn fixed_mul_floor_integrity(e: &Env) {
    let x = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= x && x <= i64::MAX as i128);
    clog!(x);
    let y = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= y && y <= i64::MAX as i128);
    clog!(y);
    let z = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= z && z <= i64::MAX as i128);
    clog!(z);
    let result = x.fixed_mul_floor(e, &y, &z);
    clog!(result);
    let expected_result = x * y / z;
    clog!(expected_result);
    let max_rounding_error: i128 = 1;
    clog!(max_rounding_error);
    let lower_bound = expected_result.checked_sub(max_rounding_error).unwrap();
    cvlr_assert!(result <= expected_result);
    cvlr_assert!(result >= lower_bound);
}

#[rule]
// result is at least expected
// status: verified https://prover.certora.com/output/33158/cee6ba53f3f14674851955c8aec9a0c1
// sanity: https://prover.certora.com/output/33158/465304f6d768450d8660b8c25290c0ee
// NOTE: see the usage of certora feature in `i128_fixed_point.rs` that skips the 256 bit attempt due to 64 bit assumptions here
pub fn fixed_mul_ceil_integrity(e: &Env) {
    let x = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= x && x <= i64::MAX as i128);
    clog!(x);
    let y = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= y && y <= i64::MAX as i128);
    clog!(y);
    let z = i128::nondet();
    cvlr_assume!(i64::MIN as i128 <= z && z <= i64::MAX as i128);
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

