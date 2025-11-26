use cvlr::{cvlr_assert,cvlr_assume,cvlr_satisfy};use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};
use crate::math::i128_fixed_point::*;
use crate::math::fixed_point::Rounding;
use crate::math::soroban_fixed_point::SorobanFixedPoint;
use soroban_sdk::I256;
use cvlr::clog;

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
 
 #[rule]
 // fixed_mul_floor panics if the result overflows
 // status: violated - spurious?
 pub fn fixed_mul_floor_panics_if_result_overflows(e: &Env) {
    let x = i128::nondet(); 
    clog!(x);
    let y = i128::nondet();
    clog!(y);
    let z = i128::nondet();
    clog!(z);
    let x_256 = I256::from_i128(e, x);
    let y_256 = I256::from_i128(e, y);
    let z_256 = I256::from_i128(e, z);
    let result = x_256.mul(&y_256).div(&z_256);
    cvlr_assume!(result > I256::from_i128(e, i128::MAX));
    let result = x.fixed_mul_floor(e, &y, &z);
    clog!(result);
    cvlr_assert!(false);
 }


 #[rule]
 // fixed_mul_ceil panics if the result overflows
 // status: violated - spurious?
 pub fn fixed_mul_ceil_panics_if_result_overflows(e: &Env) {
    let x = i128::nondet();
    clog!(x);
    let y = i128::nondet();
    clog!(y);
    let z = i128::nondet();
    clog!(z);
    let x_256 = I256::from_i128(e, x);
    let y_256 = I256::from_i128(e, y);
    let z_256 = I256::from_i128(e, z);
    let result = x_256.mul(&y_256).div(&z_256);
    cvlr_assume!(result > I256::from_i128(e, i128::MAX));
    let result = x.fixed_mul_ceil(e, &y, &z);
    clog!(result);
    cvlr_assert!(false);
 }