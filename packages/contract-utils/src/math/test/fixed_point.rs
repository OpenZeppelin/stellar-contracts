#![cfg(test)]

extern crate std;

use soroban_sdk::Env;

use crate::math::{muldiv, Rounding, SorobanFixedPoint};

#[test]
fn test_fixed_mul_floor_rounds_down() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, 483_5313675)
}

#[test]
fn test_fixed_mul_floor_negative_rounds_down() {
    let env = Env::default();
    let x: i128 = -1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, -483_5313676)
}

#[test]
fn test_fixed_mul_floor_phantom_overflow_scales() {
    let env = Env::default();
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 10i128.pow(27);
    let denominator: i128 = 10i128.pow(18);

    let result = x.fixed_mul_floor(&env, &y, &denominator);

    assert_eq!(result, 170_141_183_460_469_231_731 * 10i128.pow(9));
}

/********** fixed_mul_ceil ********* */

#[test]
fn test_fixed_mul_ceil_rounds_up() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, 483_5313676)
}

#[test]
fn test_fixed_mul_ceil_negative_rounds_up() {
    let env = Env::default();
    let x: i128 = -1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, -483_5313675)
}

#[test]
fn test_fixed_mul_ceil_large_number() {
    let env = Env::default();
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 1_000_000_000_000_000_000;
    let denominator: i128 = 1_000_000_000_000_000_000;

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, 170_141_183_460_469_231_731)
}

#[test]
fn test_fixed_mul_ceil_phantom_overflow_scales() {
    let env = Env::default();
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 10i128.pow(27);
    let denominator: i128 = 10i128.pow(18);

    let result = x.fixed_mul_ceil(&env, &y, &denominator);

    assert_eq!(result, 170_141_183_460_469_231_731 * 10i128.pow(9));
}

#[test]
fn test_muldiv_floor_rounds_down() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, 483_5313675);
}

#[test]
fn test_muldiv_ceil_rounds_up() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Ceil);

    assert_eq!(result, 483_5313676);
}

#[test]
fn test_muldiv_floor_negative() {
    let env = Env::default();
    let x: i128 = -1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, -483_5313676);
}

#[test]
fn test_muldiv_ceil_negative() {
    let env = Env::default();
    let x: i128 = -1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Ceil);

    assert_eq!(result, -483_5313675);
}

#[test]
fn test_muldiv_exact_division() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 10;

    let result_floor = muldiv(&env, x, y, denominator, Rounding::Floor);
    let result_ceil = muldiv(&env, x, y, denominator, Rounding::Ceil);

    assert_eq!(result_floor, 500);
    assert_eq!(result_ceil, 500);
}

#[test]
fn test_muldiv_with_zero_x() {
    let env = Env::default();
    let x: i128 = 0;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, 0);
}

#[test]
fn test_muldiv_with_zero_y() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 0;
    let denominator: i128 = 1_0000001;

    let result = muldiv(&env, x, y, denominator, Rounding::Ceil);

    assert_eq!(result, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_muldiv_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    muldiv(&env, x, y, denominator, Rounding::Floor);
}

#[test]
fn test_muldiv_phantom_overflow_scales() {
    let env = Env::default();
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 10i128.pow(27);
    let denominator: i128 = 10i128.pow(18);

    let result = muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, 170_141_183_460_469_231_731 * 10i128.pow(9));
}

// ################## CHECKED_MULDIV TESTS ##################

#[test]
fn test_checked_muldiv_floor_success() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, Some(483_5313675));
}

#[test]
fn test_checked_muldiv_ceil_success() {
    let env = Env::default();
    let x: i128 = 1_5391283;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Ceil);

    assert_eq!(result, Some(483_5313676));
}

#[test]
fn test_checked_muldiv_zero_denominator() {
    let env = Env::default();
    let x: i128 = 100;
    let y: i128 = 50;
    let denominator: i128 = 0;

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, None);
}

#[test]
fn test_checked_muldiv_overflow() {
    let env = Env::default();
    let x: i128 = i128::MAX;
    let y: i128 = i128::MAX;
    let denominator: i128 = 1;

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, None);
}

#[test]
fn test_checked_muldiv_phantom_overflow_handled() {
    let env = Env::default();
    // Intermediate overflow but final result fits
    let x: i128 = 170_141_183_460_469_231_731;
    let y: i128 = 10i128.pow(27);
    let denominator: i128 = 10i128.pow(18);

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, Some(170_141_183_460_469_231_731 * 10i128.pow(9)));
}

#[test]
fn test_checked_muldiv_with_zero_values() {
    let env = Env::default();
    let x: i128 = 0;
    let y: i128 = 314_1592653;
    let denominator: i128 = 1_0000001;

    let result = crate::math::checked_muldiv(&env, x, y, denominator, Rounding::Floor);

    assert_eq!(result, Some(0));
}
