#![cfg(test)]

extern crate std;

use soroban_sdk::{Env, I256};

use crate::math::SorobanFixedPoint;

#[test]
fn test_fixed_mul_floor_rounds_down() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 483_5313675));
}

#[test]
fn test_fixed_mul_floor_negative_rounds_down() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -483_5313676));
}

#[test]
fn test_fixed_mul_floor_large_number() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = x.clone().fixed_mul_floor(&env, &y, &denominator);

    let expected_result = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, expected_result);
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn test_fixed_mul_floor_phantom_overflow() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    // 256 bit max ~= 5.8e76, 128 bit max ~= 1.7e38, need to multiply by at least
    // 10^39
    let y: I256 = I256::from_i128(&env, 10i128.pow(39));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    x.fixed_mul_floor(&env, &y, &denominator);
}

/********** fixed_mul_ceil ********* */

#[test]
fn test_fixed_mul_ceil_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 483_5313676));
}

#[test]
fn test_fixed_mul_ceil_negative_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -483_5313675));
}

#[test]
fn test_fixed_mul_ceil_large_number() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = x.clone().fixed_mul_ceil(&env, &y, &denominator);

    let expected_result = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, expected_result);
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn test_fixed_mul_ceil_phantom_overflow() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    // 256 bit max ~= 5.8e76, 128 bit max ~= 1.7e38, need to multiply by at least
    // 10^39
    let y: I256 = I256::from_i128(&env, 10i128.pow(39));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    x.fixed_mul_ceil(&env, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_fixed_mul_floor_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    x.fixed_mul_floor(&env, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_fixed_mul_ceil_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    x.fixed_mul_ceil(&env, &y, &denominator);
}

#[test]
fn test_fixed_mul_floor_with_zero_x() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 0);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
fn test_fixed_mul_ceil_with_zero_y() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 0);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
fn test_fixed_mul_floor_exact_division() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 500));
}

#[test]
fn test_fixed_mul_ceil_exact_division() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 500));
}

#[test]
fn test_fixed_mul_floor_one_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 123_456_789);
    let y: I256 = I256::from_i128(&env, 987_654_321);
    let denominator: I256 = I256::from_i128(&env, 1);

    let result = x.clone().fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, x.mul(&y));
}

#[test]
fn test_fixed_mul_ceil_one_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 123_456_789);
    let y: I256 = I256::from_i128(&env, 987_654_321);
    let denominator: I256 = I256::from_i128(&env, 1);

    let result = x.clone().fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, x.mul(&y));
}

#[test]
fn test_fixed_mul_floor_negative_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_fixed_mul_ceil_negative_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_fixed_mul_floor_all_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -100);
    let y: I256 = I256::from_i128(&env, -50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_fixed_mul_ceil_all_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -100);
    let y: I256 = I256::from_i128(&env, -50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

// ################## CHECKED VARIANTS ##################

#[test]
fn test_checked_fixed_mul_floor_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, 500)));
}

#[test]
fn test_checked_fixed_mul_floor_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    let result = x.checked_fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_fixed_mul_floor_large_numbers() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = x.clone().checked_fixed_mul_floor(&env, &y, &denominator);

    let expected = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, Some(expected));
}

#[test]
fn test_checked_fixed_mul_ceil_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, 483_5313676)));
}

#[test]
fn test_checked_fixed_mul_ceil_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_fixed_mul_ceil_negative_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = x.checked_fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, -483_5313675)));
}
