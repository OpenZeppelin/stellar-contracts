use cvlr::{cvlr_assert,cvlr_satisfy};use cvlr_soroban_derive::rule;
use cvlr::nondet::Nondet;

use soroban_sdk::{Env};
use crate::math::i128_fixed_point::*;
use crate::math::fixed_point::Rounding;
use crate::math::soroban_fixed_point::SorobanFixedPoint;
// TODO: need 256 support

#[rule]
pub fn div_floor_sanity() {
    let r = i128::nondet();
    let z = i128::nondet();
    let _ = div_floor(r, z);
    cvlr_satisfy!(true);
}

#[rule]
pub fn div_ceil_sanity() {
    let r = i128::nondet();
    let z = i128::nondet();
    let _ = div_ceil(r, z);
    cvlr_satisfy!(true);
}

#[rule]
pub fn scaled_mul_div_floor_sanity(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    let _ = scaled_mul_div_floor(&x, &e, &y, &z);
    cvlr_satisfy!(true);
}

#[rule]
pub fn scaled_mul_div_ceil_sanity(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    let _ = scaled_mul_div_ceil(&x, &e, &y, &z);
    cvlr_satisfy!(true);
}

#[rule]
pub fn fixed_mul_floor_sanity(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    let _ = x.fixed_mul_floor(e, &y, &z);
    cvlr_satisfy!(true);
}

#[rule]
pub fn fixed_mul_ceil_sanity(e: &Env) {
    let x = i128::nondet();
    let y = i128::nondet();
    let z = i128::nondet();
    let _ = x.fixed_mul_ceil(e, &y, &z);
    cvlr_satisfy!(true);
}