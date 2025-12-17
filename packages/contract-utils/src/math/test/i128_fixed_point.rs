#![cfg(test)]

extern crate std;

use soroban_sdk::Env;

use crate::math::SorobanFixedPoint;

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_fixed_mul_floor_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    x.fixed_mul_floor(&env, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_fixed_mul_ceil_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    x.fixed_mul_ceil(&env, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_fixed_mul_floor_result_overflow() {
    let env = Env::default();
    // This will overflow i128 even after scaling to I256
    let x: i128 = i128::MAX;
    let y: i128 = i128::MAX;
    let denominator: i128 = 1;

    x.fixed_mul_floor(&env, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_fixed_mul_ceil_result_overflow() {
    let env = Env::default();
    // This will overflow i128 even after scaling to I256
    let x: i128 = i128::MAX;
    let y: i128 = i128::MAX;
    let denominator: i128 = 1;

    x.fixed_mul_ceil(&env, &y, &denominator);
}

#[test]
fn test_fixed_mul_floor_with_zero_x() {
    let env = Env::default();
    let x: i128 = 0;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, 0);
}

#[test]
fn test_fixed_mul_ceil_with_zero_y() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 0;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, 0);
}

#[test]
fn test_fixed_mul_floor_exact_division() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 10;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, 500);
}

#[test]
fn test_fixed_mul_ceil_exact_division() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 10;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, 500);
}

#[test]
fn test_fixed_mul_floor_one_denominator() {
    let env = Env::default();
    let x: i128 = 123_456_789;
    let y: i128 = 987_654_321;
    let denominator: i128 = 1;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, x * y);
}

#[test]
fn test_fixed_mul_ceil_one_denominator() {
    let env = Env::default();
    let x: i128 = 123_456_789;
    let y: i128 = 987_654_321;
    let denominator: i128 = 1;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, x * y);
}

#[test]
fn test_fixed_mul_floor_negative_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = -10;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, -500);
}

#[test]
fn test_fixed_mul_ceil_negative_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = -10;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, -500);
}

#[test]
fn test_fixed_mul_floor_all_negative() {
    let env = Env::default();
    let x: i128 = -100;
    let y: i128 = -50;
    let denominator: i128 = -10;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, -500);
}

#[test]
fn test_fixed_mul_ceil_all_negative() {
    let env = Env::default();
    let x: i128 = -100;
    let y: i128 = -50;
    let denominator: i128 = -10;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, -500);
}

// ################## CHECKED VARIANTS ##################

#[test]
fn test_checked_fixed_mul_floor_success() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 10;

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, Some(500));
}

#[test]
fn test_checked_fixed_mul_floor_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_fixed_mul_floor_overflow() {
    let env = Env::default();
    let x: i128 = i128::MAX;
    let y: i128 = i128::MAX;
    let denominator: i128 = 1;

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_fixed_mul_floor_phantom_overflow_handled() {
    let env = Env::default();
    // Intermediate overflow but final result fits
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 10i128.pow(27);
    let denominator: i128 = 10i128.pow(18);

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, Some(170_141_183_460_469_231_731 * 10i128.pow(9)));
}

#[test]
fn test_checked_fixed_mul_ceil_success() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, Some(483_5313676));
}

#[test]
fn test_checked_fixed_mul_ceil_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_fixed_mul_ceil_overflow() {
    let env = Env::default();
    let x: i128 = i128::MAX;
    let y: i128 = i128::MAX;
    let denominator: i128 = 1;

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_div_floor_both_negative() {
    let env = Env::default();
    let x: i128 = 1;
    let y: i128 = -5;
    let z: i128 = -2;

    // r = -5, r / -2 = floor(2.5) = 2
    let result = x.fixed_mul_floor(&env, &y, &z);

    assert_eq!(result, 2);
}

#[test]
fn test_div_floor_r_negative_z_positive() {
    let env = Env::default();
    let x: i128 = 1;
    let y: i128 = -5;
    let z: i128 = 2;

    // r = -5, r / 2 = floor(-2.5) = -3
    let result = x.fixed_mul_floor(&env, &y, &z);

    assert_eq!(result, -3);
}

#[test]
fn test_div_ceil_both_negative() {
    let env = Env::default();
    let x: i128 = 1;
    let y: i128 = -5;
    let z: i128 = -2;

    // r = -5, r / -2 = ceil(2.5) = 3
    let result = x.fixed_mul_ceil(&env, &y, &z);

    assert_eq!(result, 3);
}

#[test]
fn test_div_ceil_r_negative_z_positive() {
    let env = Env::default();
    let x: i128 = 1;
    let y: i128 = -5;
    let z: i128 = 2;

    // r = -5, r / 2 = ceil(-2.5) = -2
    let result = x.fixed_mul_ceil(&env, &y, &z);

    assert_eq!(result, -2);
}
