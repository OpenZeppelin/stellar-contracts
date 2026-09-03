#![cfg(test)]

extern crate std;

use std::panic::{catch_unwind, AssertUnwindSafe};

use proptest::prelude::*;
use soroban_sdk::{Bytes, Env, I256};

use crate::math::{
    i256_fixed_point::{
        checked_mul_div, checked_mul_div_ceil, checked_mul_div_decomposed, checked_mul_div_floor,
        checked_mul_div_with_rounding, mul_div, mul_div_ceil, mul_div_floor, mul_div_with_rounding,
    },
    Rounding,
};

#[test]
#[should_panic(expected = "Error(Object, ArithDomain)")]
fn test_mul_div_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    mul_div(&x, &y, &denominator);
}

#[test]
fn test_checked_mul_div_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    let result = checked_mul_div(&x, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_mul_div_div_overflow_returns_none() {
    let env = Env::default();
    let i256_min = I256::min_value(&env);
    let one = I256::from_i128(&env, 1);
    let neg_one = I256::from_i128(&env, -1);

    // I256::MIN * 1 / -1 would overflow (result is I256::MAX + 1),
    // checked variant must return None instead of panicking.
    let result = checked_mul_div(&i256_min, &one, &neg_one);
    assert_eq!(result, None);

    // Also verify via checked_mul_div_with_rounding with Truncate rounding,
    // which dispatches to checked_mul_div.
    let result = checked_mul_div_with_rounding(i256_min, one, neg_one, Rounding::Truncate);
    assert_eq!(result, None);
}

#[test]
fn test_mul_div_floor_rounds_down() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 483_5313675));
}

#[test]
fn test_mul_div_floor_negative_rounds_down() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -483_5313676));
}

#[test]
fn test_mul_div_floor_large_number() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = mul_div_floor(&x, &y, &denominator);

    let expected_result = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, expected_result);
}

#[test]
fn test_mul_div_floor_phantom_overflow_resolves() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    // x * 10^39 overflows the I256 intermediate (max ~5.8e76), so this
    // exercises the fallback. The quotient is 10^21 * x, which fits, so it
    // is recovered rather than failing.
    //
    // 10^39 is the multiplier we need, but it exceeds i128::MAX (~1.7e38), so
    // it cannot be written as `10i128.pow(39)`. It is built in I256 space
    // instead.
    let y: I256 = I256::from_i128(&env, 10i128.pow(20)).mul(&I256::from_i128(&env, 10i128.pow(19)));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, x.mul(&I256::from_i128(&env, 10i128.pow(21))));
}

#[test]
fn test_mul_div_ceil_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 483_5313676));
}

#[test]
fn test_mul_div_ceil_negative_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -483_5313675));
}

#[test]
fn test_mul_div_ceil_large_number() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = mul_div_ceil(&x, &y, &denominator);

    let expected_result = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, expected_result);
}

#[test]
fn test_mul_div_ceil_phantom_overflow_resolves() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    // Same input as the floor case. The division is exact, so ceil returns the
    // same value.
    //
    // 10^39 exceeds i128::MAX (~1.7e38), so it is built in I256 space rather
    // than as `10i128.pow(39)`.
    let y: I256 = I256::from_i128(&env, 10i128.pow(20)).mul(&I256::from_i128(&env, 10i128.pow(19)));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, x.mul(&I256::from_i128(&env, 10i128.pow(21))));
}

#[test]
#[should_panic(expected = "Error(Object, ArithDomain)")]
fn test_mul_div_floor_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    mul_div_floor(&x, &y, &denominator);
}

#[test]
#[should_panic(expected = "Error(Object, ArithDomain)")]
fn test_mul_div_ceil_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    mul_div_ceil(&x, &y, &denominator);
}

#[test]
fn test_mul_div_floor_with_zero_x() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 0);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
fn test_mul_div_ceil_with_zero_y() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 0);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
fn test_mul_div_floor_exact_division() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 500));
}

#[test]
fn test_mul_div_ceil_exact_division() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, 500));
}

#[test]
fn test_mul_div_floor_one_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 123_456_789);
    let y: I256 = I256::from_i128(&env, 987_654_321);
    let denominator: I256 = I256::from_i128(&env, 1);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, x.mul(&y));
}

#[test]
fn test_mul_div_ceil_one_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 123_456_789);
    let y: I256 = I256::from_i128(&env, 987_654_321);
    let denominator: I256 = I256::from_i128(&env, 1);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, x.mul(&y));
}

#[test]
fn test_mul_div_floor_negative_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_mul_div_ceil_negative_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_mul_div_floor_all_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -100);
    let y: I256 = I256::from_i128(&env, -50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_mul_div_ceil_all_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -100);
    let y: I256 = I256::from_i128(&env, -50);
    let denominator: I256 = I256::from_i128(&env, -10);

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, I256::from_i128(&env, -500));
}

#[test]
fn test_mul_div_ceil_both_positive() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = 5, r / 2 = 2 (truncated), ceil(2.5) = 3
    let result = mul_div_ceil(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 3));
}

#[test]
fn test_mul_div_ceil_both_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, -5);
    let z: I256 = I256::from_i128(&env, -2);

    // r = -5, r / -2 = 2 (truncated), ceil(2.5) = 3
    let result = mul_div_ceil(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 3));
}

#[test]
fn test_mul_div_ceil_r_positive_z_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, -2);

    // r = 5, r / -2 = -2 (truncated), ceil(-2.5) = -2
    let result = mul_div_ceil(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, -2));
}

#[test]
fn test_mul_div_ceil_r_negative_z_positive() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, -5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = -5, r / 2 = -2 (truncated), ceil(-2.5) = -2
    let result = mul_div_ceil(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, -2));
}

#[test]
fn test_mul_div_ceil_r_zero() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 0);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = 0, 0 / 2 = 0
    let result = mul_div_ceil(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
#[should_panic]
fn test_mul_div_ceil_z_zero() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 0);

    mul_div_ceil(&x, &y, &z);
}

#[test]
fn test_mul_div_floor_both_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, -5);
    let z: I256 = I256::from_i128(&env, -2);

    // r = -5, r / -2 = 2
    let result = mul_div_floor(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 2));
}

#[test]
fn test_mul_div_floor_both_positive() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = 5, r / 2 = 2
    let result = mul_div_floor(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 2));
}

#[test]
fn test_mul_div_floor_r_positive_z_negative() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, -2);

    // r = 5, r / -2 = -2 (truncated), floor(-2.5) = -3
    let result = mul_div_floor(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, -3));
}

#[test]
fn test_mul_div_floor_r_negative_z_positive() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, -5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = -5, r / 2 = -2 (truncated), floor(-2.5) = -3
    let result = mul_div_floor(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, -3));
}

#[test]
fn test_mul_div_floor_r_zero() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 0);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 2);

    // r = 0, 0 / 2 = 0
    let result = mul_div_floor(&x, &y, &z);

    assert_eq!(result, I256::from_i128(&env, 0));
}

#[test]
#[should_panic]
fn test_mul_div_floor_z_zero() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1);
    let y: I256 = I256::from_i128(&env, 5);
    let z: I256 = I256::from_i128(&env, 0);

    mul_div_floor(&x, &y, &z);
}

// ################## CHECKED VARIANTS ##################

#[test]
fn test_checked_mul_div_floor_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 10);

    let result = checked_mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, 500)));
}

#[test]
fn test_checked_mul_div_floor_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    let result = checked_mul_div_floor(&x, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_mul_div_floor_large_numbers() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, i128::MAX);
    let y: I256 = I256::from_i128(&env, 10i128.pow(38));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = checked_mul_div_floor(&x, &y, &denominator);

    let expected = x.mul(&I256::from_i128(&env, 10i128.pow(20)));
    assert_eq!(result, Some(expected));
}

#[test]
fn test_checked_mul_div_ceil_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = checked_mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, 483_5313676)));
}

#[test]
fn test_checked_mul_div_ceil_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    let result = checked_mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, None);
}

#[test]
fn test_checked_mul_div_ceil_negative_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, -1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = checked_mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, Some(I256::from_i128(&env, -483_5313675)));
}

#[test]
fn test_checked_mul_div_floor_negative_with_remainder() {
    let env = Env::default();
    // Choose r = x * y negative and not divisible by z
    let x: I256 = I256::from_i128(&env, -7);
    let y: I256 = I256::from_i128(&env, 10);
    let z: I256 = I256::from_i128(&env, 3);

    // r = -70, r / 3 = -23
    // r < 0, remainder > 0 -> result = r.div(z) - 1 = -24
    let result = checked_mul_div_floor(&x, &y, &z).unwrap();

    assert_eq!(result, I256::from_i128(&env, -24));
}

#[test]
fn test_checked_mul_div_mul_overflow_resolves() {
    let env = Env::default();
    // x * y = I256::MAX * 2 overflows I256, but the quotient is exactly
    // I256::MAX. The remainders are both zero, so the decomposition
    // recovers the answer and the checked variants return `Some` where they
    // used to return `None`.
    let x: I256 = I256::max_value(&env);
    let y: I256 = I256::from_i128(&env, 2);
    let denominator: I256 = I256::from_i128(&env, 2);
    let expected: I256 = I256::max_value(&env);

    assert_eq!(checked_mul_div(&x, &y, &denominator), Some(expected.clone()));
    assert_eq!(checked_mul_div_floor(&x, &y, &denominator), Some(expected.clone()));
    assert_eq!(checked_mul_div_ceil(&x, &y, &denominator), Some(expected.clone()));
    assert_eq!(
        checked_mul_div_with_rounding(x.clone(), y.clone(), denominator.clone(), Rounding::Floor),
        Some(expected.clone())
    );
    assert_eq!(
        checked_mul_div_with_rounding(x.clone(), y.clone(), denominator.clone(), Rounding::Ceil),
        Some(expected.clone())
    );
    assert_eq!(
        checked_mul_div_with_rounding(x, y, denominator, Rounding::Truncate),
        Some(expected)
    );
}

#[test]
#[should_panic(expected = "Error(Object, ArithDomain)")]
fn test_mul_div_min_by_negative_one_panics() {
    let env = Env::default();
    // `I256::MIN / -1` is `2^255`, which has no `I256` representation.
    // `checked_mul` succeeds and the overflow happens inside the unchecked
    // division, so this is the one failure the module reports as the host's
    // arithmetic error rather than a contract error code. Pinned deliberately
    // rather than fixed: the i128 sibling has the identical trap, and routing
    // the fast path through the checked helper would re-price every
    // non-overflowing call.
    let x: I256 = I256::min_value(&env);
    let y: I256 = I256::from_i128(&env, 1);
    let denominator: I256 = I256::from_i128(&env, -1);

    mul_div_floor(&x, &y, &denominator);
}

#[test]
fn test_checked_mul_div_min_by_negative_one_returns_none() {
    let env = Env::default();
    // The checked counterpart of the case above. Both fail, which is what keeps
    // the two families in agreement on their domain; only the reporting
    // differs.
    let x: I256 = I256::min_value(&env);
    let y: I256 = I256::from_i128(&env, 1);
    let denominator: I256 = I256::from_i128(&env, -1);

    assert_eq!(checked_mul_div_floor(&x, &y, &denominator), None);
    assert_eq!(checked_mul_div_ceil(&x, &y, &denominator), None);
    assert_eq!(checked_mul_div(&x, &y, &denominator), None);
}

// ################## MULDIV TESTS ##################

#[test]
fn test_muldiv_floor_rounds_down() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_with_rounding(x, y, denominator, Rounding::Floor);

    assert_eq!(result, I256::from_i128(&env, 483_5313675));
}

#[test]
fn test_muldiv_ceil_rounds_up() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_with_rounding(x, y, denominator, Rounding::Ceil);

    assert_eq!(result, I256::from_i128(&env, 483_5313676));
}

#[test]
fn test_muldiv_truncate() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = mul_div_with_rounding(x, y, denominator, Rounding::Truncate);

    assert_eq!(result, I256::from_i128(&env, 483_5313675));
}

// ################## CHECKED_MULDIV TESTS ##################

#[test]
fn test_checked_muldiv_floor_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = checked_mul_div_with_rounding(x, y, denominator, Rounding::Floor);

    assert_eq!(result, Some(I256::from_i128(&env, 483_5313675)));
}

#[test]
fn test_checked_muldiv_ceil_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = checked_mul_div_with_rounding(x, y, denominator, Rounding::Ceil);

    assert_eq!(result, Some(I256::from_i128(&env, 483_5313676)));
}

#[test]
fn test_checked_muldiv_truncate_success() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 1_5391283);
    let y: I256 = I256::from_i128(&env, 314_1592653);
    let denominator: I256 = I256::from_i128(&env, 1_0000001);

    let result = checked_mul_div_with_rounding(x, y, denominator, Rounding::Truncate);

    assert_eq!(result, Some(I256::from_i128(&env, 483_5313675)));
}

// ################## FALLBACK HELPERS ##################

/// Shorthand for the small `I256` literals the fallback tests are built from.
fn i(e: &Env, v: i128) -> I256 {
    I256::from_i128(e, v)
}

/// `2^n` as an `I256`.
fn two_pow(e: &Env, n: u32) -> I256 {
    I256::from_i32(e, 2).pow(n)
}

/// `-v`. The SDK exposes no `Neg` impl, and nothing below negates `I256::MIN`.
fn negate(v: &I256) -> I256 {
    I256::from_i32(v.env(), 0).sub(v)
}

/// The unchecked half of one family pair, held as a function pointer so all
/// three rounding modes can be driven from one table.
type UncheckedMulDiv = fn(&I256, &I256, &I256) -> I256;

/// Reads 32 bytes as an `I256`, the route the property tests use because `I256`
/// has no `Arbitrary` impl.
fn from_be(e: &Env, bytes: &[u8; 32]) -> I256 {
    I256::from_be_bytes(e, &Bytes::from_array(e, bytes))
}

/// Builds a signed `I256` from the low `significant` bytes of `bytes`, so the
/// property tests can bound operand size without rejecting draws.
fn bounded(e: &Env, bytes: &[u8; 32], significant: usize, negative: bool) -> I256 {
    let mut buf = [0u8; 32];
    buf[32 - significant..].copy_from_slice(&bytes[32 - significant..]);
    let value = from_be(e, &buf);
    if negative {
        negate(&value)
    } else {
        value
    }
}

/// A property-test operand, with magnitude in `2^151 ..= 2^152 - 1`.
///
/// Both bounds matter. The floor is forced so that two operands always multiply
/// past `2^255`, which puts every draw on the fallback rather than most of
/// them; each property test asserts that the fast path did give up. The ceiling
/// keeps the quotient representable, and keeps `I256::MIN` out of range, so
/// negating an operand or a result is always defined.
fn operand(e: &Env, bytes: &[u8; 32], negative: bool) -> I256 {
    let mut buf = *bytes;
    buf[13] |= 0x80;
    bounded(e, &buf, 19, negative)
}

/// A property-test denominator, with magnitude in `2^127 ..= 2^128 - 1`.
///
/// The high bit is forced so that no draw is small enough to push the quotient
/// out of `I256`, and the ceiling keeps every draw inside the guaranteed
/// domain.
fn denominator(e: &Env, bytes: &[u8; 32], negative: bool) -> I256 {
    let mut buf = *bytes;
    buf[16] |= 0x80;
    bounded(e, &buf, 16, negative)
}

/// Drives one magnitude shape through all eight sign combinations and all three
/// rounding modes.
///
/// `magnitude` is the truncated magnitude of the exact quotient and `inexact`
/// says whether anything was dropped. Together they fix every expected value:
/// truncation keeps the magnitude, floor moves away from zero only for a
/// negative result, ceil only for a positive one, and neither moves when the
/// quotient is exact.
fn assert_rounding_matrix(
    e: &Env,
    abs_x: &I256,
    abs_y: &I256,
    abs_d: &I256,
    magnitude: &I256,
    inexact: bool,
) {
    let zero = i(e, 0);
    let step = if inexact { i(e, 1) } else { zero.clone() };

    for signs in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let (neg_x, neg_y, neg_d) = signs;
        let x = if neg_x { negate(abs_x) } else { abs_x.clone() };
        let y = if neg_y { negate(abs_y) } else { abs_y.clone() };
        let d = if neg_d { negate(abs_d) } else { abs_d.clone() };
        let negative = neg_x ^ neg_y ^ neg_d;

        assert_eq!(x.checked_mul(&y), None, "signs {signs:?} must reach the fallback");

        let (floor, ceil) = if negative {
            (negate(&magnitude.add(&step)), negate(magnitude))
        } else {
            (magnitude.clone(), magnitude.add(&step))
        };
        let truncate = if negative { negate(magnitude) } else { magnitude.clone() };

        assert_eq!(mul_div_floor(&x, &y, &d), floor, "floor, signs {signs:?}");
        assert_eq!(mul_div_ceil(&x, &y, &d), ceil, "ceil, signs {signs:?}");
        assert_eq!(mul_div(&x, &y, &d), truncate, "truncate, signs {signs:?}");
        // The sign of the result, asserted on its own rather than read off the
        // magnitudes above.
        assert_eq!(truncate < zero, negative, "sign, signs {signs:?}");
    }
}

// ################## FALLBACK: DIFFERENTIAL ##################

#[test]
fn decomposition_matches_fast_path_works() {
    let e = Env::default();
    // Thousands of metered host calls in one `Env` exceed the default budget,
    // and the resulting `Error(Budget, ExceededLimit)` names no test logic
    // at all.
    e.cost_estimate().budget().reset_unlimited();

    let mut compared = 0u32;
    for x in -13i128..=13 {
        for y in -13i128..=13 {
            for d in -6i128..=6 {
                // Production reaches the fallback only with `|x * y| >= 2^255
                // >= |d|`, so the result magnitude is at least
                // one. Calling the core directly outside that
                // band asks `from_magnitude` for a negatively signed
                // zero, which it answers `None` to by design.
                if d == 0 || x == 0 || y == 0 || x.abs() * y.abs() < d.abs() {
                    continue;
                }
                let (xi, yi, di) = (i(&e, x), i(&e, y), i(&e, d));
                for (rounding, fast) in [
                    (Rounding::Floor, mul_div_floor(&xi, &yi, &di)),
                    (Rounding::Ceil, mul_div_ceil(&xi, &yi, &di)),
                    (Rounding::Truncate, mul_div(&xi, &yi, &di)),
                ] {
                    let slow = checked_mul_div_decomposed(&xi, &yi, &di, rounding)
                        .expect("small operands are inside the fallback's domain");
                    assert_eq!(slow, fast, "x={x} y={y} d={d}");
                    compared += 1;
                }
            }
        }
    }

    assert!(compared > 3_000, "the grid degenerated to {compared} comparisons");
}

#[test]
fn prop_decomposition_matches_fast_path() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(x: i128, y: i128, d: i128)| {
        // `|i128 * i128| <= 2^254`, so the fast path is defined for every draw here
        // and the two routes are directly comparable.
        prop_assume!(x != 0 && y != 0 && d != 0);
        // Same lower bound on the result magnitude as the grid above.
        let in_band = match x.unsigned_abs().checked_mul(y.unsigned_abs()) {
            Some(product) => product >= d.unsigned_abs(),
            // A product past `u128` exceeds every possible `|d|`.
            None => true,
        };
        prop_assume!(in_band);

        let (xi, yi, di) = (i(&e, x), i(&e, y), i(&e, d));
        prop_assert_eq!(
            checked_mul_div_decomposed(&xi, &yi, &di, Rounding::Floor),
            Some(mul_div_floor(&xi, &yi, &di))
        );
        prop_assert_eq!(
            checked_mul_div_decomposed(&xi, &yi, &di, Rounding::Ceil),
            Some(mul_div_ceil(&xi, &yi, &di))
        );
        prop_assert_eq!(
            checked_mul_div_decomposed(&xi, &yi, &di, Rounding::Truncate),
            Some(mul_div(&xi, &yi, &di))
        );
    })
}

// ################## FALLBACK: DOMAIN BOUNDARY ##################

#[test]
fn mul_div_at_denominator_two_pow_128_works() {
    let e = Env::default();
    let d = two_pow(&e, 128);
    let x = d.clone();
    let y = d.add(&i(&e, 1));

    // `x * y` is `2^256 + 2^128`, so the fast path gives up.
    assert_eq!(x.checked_mul(&y), None);

    // `2^128 * (2^128 + 1) / 2^128` is `2^128 + 1` exactly, so all three modes
    // agree.
    let expected = d.add(&i(&e, 1));
    assert_eq!(mul_div_floor(&x, &y, &d), expected);
    assert_eq!(mul_div_ceil(&x, &y, &d), expected);
    assert_eq!(mul_div(&x, &y, &d), expected);
}

#[test]
fn mul_div_at_maximal_remainders_works() {
    let e = Env::default();
    // The `2^128` domain rests entirely on `r1 * r2` staying inside `U256`, and
    // boundary coverage built from zero remainders never exercises that term at
    // all. Here both remainders are maximal. With `n = 2^128`, `d = n - 1` and
    // `x = y = 2n - 3`, the split is `q1 = q2 = 1` and `r1 = r2 = n - 2`, so
    // `r1 * r2 = n^2 - 4n + 4`, which is `2^256 - 2^130 + 4`: twice what `I256`
    // holds and just inside `U256`.
    //
    // The expected value comes from polynomial division rather than from the
    // decomposition, so the oracle owes nothing to the code under test:
    //
    // ```text
    // x * y = 4n^2 - 12n + 9 = (n - 1)(4n - 8) + 1
    // ```
    //
    // The quotient is `4n - 8`, which is `2^130 - 8`, and the remainder is 1,
    // so the result is inexact.
    let n = two_pow(&e, 128);
    let d = n.sub(&i(&e, 1));
    let x = n.mul(&i(&e, 2)).sub(&i(&e, 3));

    assert_eq!(x.checked_mul(&x), None);

    let floor = two_pow(&e, 130).sub(&i(&e, 8));
    assert_eq!(mul_div_floor(&x, &x, &d), floor);
    assert_eq!(mul_div(&x, &x, &d), floor);
    assert_eq!(mul_div_ceil(&x, &x, &d), floor.add(&i(&e, 1)));
}

#[test]
fn checked_mul_div_above_two_pow_128_returns_none() {
    let e = Env::default();
    // One past the domain, with the remainders that make the term overflow:
    // `r1 = r2 = 2^128`, so `r1 * r2` is `2^256` and the exact detector fires.
    let x = two_pow(&e, 128);
    let d = x.add(&i(&e, 1));

    assert_eq!(x.checked_mul(&x), None);
    assert_eq!(checked_mul_div_floor(&x, &x, &d), None);
    assert_eq!(checked_mul_div_ceil(&x, &x, &d), None);
    assert_eq!(checked_mul_div(&x, &x, &d), None);

    // The rejection is not confined to answers that would not fit. Here the
    // true answer is `2^60`, which fits with room to spare, and the
    // operation rejects anyway because `|denominator|` is past the ceiling.
    // That is the false rejection the `2^128` condition buys.
    let wide = two_pow(&e, 130);
    let wide_d = two_pow(&e, 200);

    assert_eq!(wide.checked_mul(&wide), None);
    assert_eq!(checked_mul_div_floor(&wide, &wide, &wide_d), None);
    assert_eq!(checked_mul_div_ceil(&wide, &wide, &wide_d), None);
    assert_eq!(checked_mul_div(&wide, &wide, &wide_d), None);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")] // SorobanFixedPointError::Overflow
fn mul_div_above_two_pow_128_panics() {
    let e = Env::default();
    let x = two_pow(&e, 128);
    let d = x.add(&i(&e, 1));

    mul_div_floor(&x, &x, &d);
}

#[test]
fn mul_div_at_practical_scales_works() {
    let e = Env::default();
    // Every fixed-point scale in practical use, each with a product that
    // overflows. Taking `y` as the denominator makes the exact answer `x`
    // with no remainder, an expected value that owes nothing to the
    // decomposition.
    let x = two_pow(&e, 200);
    for d in [
        i(&e, 10i128.pow(18)), // WAD
        i(&e, 10i128.pow(27)), // RAY
        two_pow(&e, 96),       // Q64.96
        i(&e, 10i128.pow(38)),
    ] {
        assert_eq!(x.checked_mul(&d), None, "the scale must overflow the fast path");
        assert_eq!(mul_div_floor(&x, &d, &d), x);
        assert_eq!(mul_div_ceil(&x, &d, &d), x);
        assert_eq!(mul_div(&x, &d, &d), x);
    }
}

#[test]
fn mul_div_inexact_at_wad_scale_works() {
    let e = Env::default();
    // The WAD scale with a remainder. Taking `y = d + 1` makes the exact
    // quotient `x + x/d`, so the expected value is computable on the fast
    // path, where `2^200` and `10^18` both fit comfortably.
    let d = i(&e, 10i128.pow(18));
    let x = two_pow(&e, 200);
    let y = d.add(&i(&e, 1));

    assert_eq!(x.checked_mul(&y), None);
    // `10^18` carries a factor of `5^18`, so it cannot divide a power of two.
    assert!(x.rem_euclid(&d) > i(&e, 0));

    let floor = x.add(&x.div(&d));
    assert_eq!(mul_div_floor(&x, &y, &d), floor);
    assert_eq!(mul_div(&x, &y, &d), floor);
    assert_eq!(mul_div_ceil(&x, &y, &d), floor.add(&i(&e, 1)));
}

#[test]
fn mul_div_beyond_guaranteed_domain_works() {
    let e = Env::default();
    // `2^128` is a sufficient domain, not a necessary one: a denominator far
    // above it still succeeds when the remainders stay small. Here `|d|` is
    // `2^255`, the largest magnitude an `I256` has, both quotients are
    // zero, and `r1 * r2` is only `2^255`.
    //
    // It is also the smallest result the fallback can produce. Entering it
    // requires `|x * y| >= 2^255`, and no `|denominator|` exceeds `2^255`,
    // so the magnitude is at least one. This input sits exactly on that
    // floor and carries a negative sign, the pairing `from_magnitude`
    // cannot represent for a zero magnitude.
    let x = two_pow(&e, 128);
    let y = two_pow(&e, 127);
    let d = I256::min_value(&e);

    assert_eq!(x.checked_mul(&y), None);

    let expected = i(&e, -1);
    assert_eq!(mul_div_floor(&x, &y, &d), expected);
    assert_eq!(mul_div_ceil(&x, &y, &d), expected);
    assert_eq!(mul_div(&x, &y, &d), expected);
}

#[test]
fn mul_div_at_denominator_max_works() {
    let e = Env::default();
    // The input the design's failure taxonomy cites to show that `2^128` is a
    // sufficient domain rather than a necessary one. `|denominator|` is `2^255
    // - 1` here, far above the ceiling, but the split leaves `r1 = r2 = 0`,
    // so `r1 * r2` never grows and the exact answer comes back.
    //
    // A different term profile from `mul_div_beyond_guaranteed_domain_works`,
    // where both quotients are zero and the whole answer comes from the
    // fractional term. Here `q1 = q2 = 1` and both remainders are zero, so
    // it all comes from `q1 * q2 * abs_d`.
    let max = I256::max_value(&e);

    assert_eq!(max.checked_mul(&max), None);

    assert_eq!(mul_div_floor(&max, &max, &max), max);
    assert_eq!(mul_div_ceil(&max, &max, &max), max);
    assert_eq!(mul_div(&max, &max, &max), max);
    assert_eq!(checked_mul_div_floor(&max, &max, &max), Some(max.clone()));
    assert_eq!(checked_mul_div_ceil(&max, &max, &max), Some(max.clone()));
    assert_eq!(checked_mul_div(&max, &max, &max), Some(max.clone()));
}

// ################## FALLBACK: ROUNDING, EXACTNESS, SIGN ##################

#[test]
fn rounding_matrix_in_fallback_domain_works() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // Inexact shape. With `d = 3`, `a = 2^127`, `b = 2^126`:
    //
    // ```text
    // x = 3a + 1, y = 3b + 2   ->   x * y = 9ab + 6a + 3b + 2
    // x * y / 3 = 3ab + 2a + b + 2/3
    // ```
    //
    // so the truncated magnitude is `3ab + 2a + b` and two thirds are dropped.
    let a = two_pow(&e, 127);
    let b = two_pow(&e, 126);
    let three = i(&e, 3);
    let abs_x = three.mul(&a).add(&i(&e, 1));
    let abs_y = three.mul(&b).add(&i(&e, 2));
    let magnitude = three.mul(&a).mul(&b).add(&i(&e, 2).mul(&a)).add(&b);
    assert_rounding_matrix(&e, &abs_x, &abs_y, &three, &magnitude, true);

    // Exact shape, chosen so that neither remainder is zero. With `d = 6`, `r1
    // = 2` and `r2 = 3`, `r1 * r2` is a non-zero exact multiple of `d`:
    //
    // ```text
    // x = 6a + 2, y = 6b + 3   ->   x * y = 36ab + 18a + 12b + 6
    // x * y / 6 = 6ab + 3a + 2b + 1, with nothing dropped
    // ```
    let a = two_pow(&e, 124);
    let b = two_pow(&e, 127);
    let six = i(&e, 6);
    let abs_x = six.mul(&a).add(&i(&e, 2));
    let abs_y = six.mul(&b).add(&i(&e, 3));
    let magnitude =
        six.mul(&a).mul(&b).add(&i(&e, 3).mul(&a)).add(&i(&e, 2).mul(&b)).add(&i(&e, 1));
    assert_rounding_matrix(&e, &abs_x, &abs_y, &six, &magnitude, false);
}

#[test]
fn exactness_with_non_zero_remainders_works() {
    let e = Env::default();
    // The case that separates "exact" from "no remainders", and the one a suite
    // whose exact cases all set `r1 = 0` or `r2 = 0` never reaches. With `d =
    // 6`, `r1 = 2` and `r2 = 3`, `r1 * r2` is 6, so the quotient is exact even
    // though neither remainder is zero. An implementation reading `r1 * r2 !=
    // 0` as inexact makes ceil round up by one here.
    let a = two_pow(&e, 125);
    let b = two_pow(&e, 126);
    let six = i(&e, 6);
    let x = six.mul(&a).add(&i(&e, 2));
    let y = six.mul(&b).add(&i(&e, 3));

    assert_eq!(x.checked_mul(&y), None);

    // `x * y / 6 = 6ab + 3a + 2b + 1`
    let expected = six.mul(&a).mul(&b).add(&i(&e, 3).mul(&a)).add(&i(&e, 2).mul(&b)).add(&i(&e, 1));

    assert_eq!(mul_div_floor(&x, &y, &six), expected);
    assert_eq!(mul_div(&x, &y, &six), expected);
    assert_eq!(mul_div_ceil(&x, &y, &six), expected, "an exact quotient must not round up");
}

#[test]
fn mul_div_negative_inexact_rounds_away_from_zero_works() {
    let e = Env::default();
    // With `d = 7`, `a = 2^126`, `x = -(7a + 3)` and `y = 7a + 5`:
    //
    // ```text
    // |x * y| = 49a^2 + 56a + 15
    // |x * y| / 7 = 7a^2 + 8a + 2 + 1/7
    // ```
    //
    // so the magnitude is `7a^2 + 8a + 2` and the result is negative and
    // inexact. Floor is the mode that moves away from zero here, which is
    // the direction that leaks value in a vault or fee path when it is
    // missed.
    let a = two_pow(&e, 126);
    let seven = i(&e, 7);
    let x = negate(&seven.mul(&a).add(&i(&e, 3)));
    let y = seven.mul(&a).add(&i(&e, 5));

    assert_eq!(x.checked_mul(&y), None);

    let truncate = negate(&seven.mul(&a).mul(&a).add(&i(&e, 8).mul(&a)).add(&i(&e, 2)));
    assert_eq!(mul_div(&x, &y, &seven), truncate);
    assert_eq!(mul_div_ceil(&x, &y, &seven), truncate);
    assert_eq!(mul_div_floor(&x, &y, &seven), truncate.sub(&i(&e, 1)));
}

#[test]
fn checked_mul_div_rounds_in_fallback_domain_works() {
    let e = Env::default();
    // Each checked function passes its own rounding mode into the fallback, and
    // nothing else here pins those three arguments to a value. The rejection
    // cases return `None` in every mode, the exact cases make all three
    // modes agree, and the dispatcher test compares a dispatcher against
    // the leaf it delegates to, so a leaf and its dispatcher move together
    // when the leaf is wrong. An inexact fallback quotient is the input
    // that separates the modes.
    //
    // With `d = 5` and `a = 2^126`:
    //
    // ```text
    // x = 5a + 3, y = 5a + 4   ->   x * y = 25a^2 + 35a + 12
    // x * y / 5 = 5a^2 + 7a + 2 + 2/5
    // ```
    let a = two_pow(&e, 126);
    let five = i(&e, 5);
    let x = five.mul(&a).add(&i(&e, 3));
    let y = five.mul(&a).add(&i(&e, 4));

    assert_eq!(x.checked_mul(&y), None);

    let magnitude = five.mul(&a).mul(&a).add(&i(&e, 7).mul(&a)).add(&i(&e, 2));

    assert_eq!(checked_mul_div_floor(&x, &y, &five), Some(magnitude.clone()));
    assert_eq!(checked_mul_div(&x, &y, &five), Some(magnitude.clone()));
    assert_eq!(checked_mul_div_ceil(&x, &y, &five), Some(magnitude.add(&i(&e, 1))));

    // Negated, where floor is the mode that moves away from zero. This half is
    // what separates floor from truncate; on a positive result the two
    // agree.
    let neg_x = negate(&x);
    let truncate = negate(&magnitude);

    assert_eq!(checked_mul_div(&neg_x, &y, &five), Some(truncate.clone()));
    assert_eq!(checked_mul_div_ceil(&neg_x, &y, &five), Some(truncate.clone()));
    assert_eq!(checked_mul_div_floor(&neg_x, &y, &five), Some(truncate.sub(&i(&e, 1))));
}

#[test]
fn mul_div_with_zero_operand_stays_on_fast_path_works() {
    let e = Env::default();
    // The fallback's sign rule is an XOR over the three operands, which matches
    // the sign of the true result only because a product involving zero
    // never overflows. A negative denominator is what would expose a
    // reliance on it: the XOR alone calls these results negative.
    let zero = i(&e, 0);
    let max = I256::max_value(&e);
    let neg_one = i(&e, -1);

    assert_eq!(zero.checked_mul(&max), Some(zero.clone()));

    for (x, y) in [(zero.clone(), max.clone()), (max.clone(), zero.clone())] {
        assert_eq!(mul_div_floor(&x, &y, &neg_one), zero);
        assert_eq!(mul_div_ceil(&x, &y, &neg_one), zero);
        assert_eq!(mul_div(&x, &y, &neg_one), zero);
        assert_eq!(checked_mul_div_floor(&x, &y, &neg_one), Some(zero.clone()));
        assert_eq!(checked_mul_div_ceil(&x, &y, &neg_one), Some(zero.clone()));
        assert_eq!(checked_mul_div(&x, &y, &neg_one), Some(zero.clone()));
    }
}

// ################## FALLBACK: REPRESENTABILITY ##################

#[test]
fn mul_div_with_min_operand_works() {
    let e = Env::default();
    // `|I256::MIN|` is `2^255`, which the type itself cannot hold, so the
    // operand reaches the core as an unsigned magnitude and has to come
    // back through the negation intact. `MIN * 3 / 3` puts `2^255` on both
    // ends of that round trip: the split is `q1 = (2^255 - 2)/3` and `r1 =
    // 2`, since `2^255 % 3 == 2`, and the terms sum back to `3*q1 + 2`,
    // which is `2^255` exactly.
    let min = I256::min_value(&e);
    let three = i(&e, 3);

    assert_eq!(min.checked_mul(&three), None);

    assert_eq!(mul_div_floor(&min, &three, &three), min);
    assert_eq!(mul_div_ceil(&min, &three, &three), min);
    assert_eq!(mul_div(&min, &three, &three), min);
}

#[test]
fn mul_div_result_at_i256_min_works() {
    let e = Env::default();
    // `I256`'s range is asymmetric, so the largest representable magnitude
    // depends on the sign: `2^255` is one past `I256::MAX` going up, but
    // exactly `I256::MIN` going down. `2^128 * 2^128 / -2` lands on it from
    // the negative side.
    //
    // The product has to be `2^256` rather than `2^255` for the fast path to
    // give up. A negative product of `2^255` is `I256::MIN`, which
    // `checked_mul` accepts.
    let x = two_pow(&e, 128);
    let d = i(&e, -2);

    assert_eq!(x.checked_mul(&x), None);

    let min = I256::min_value(&e);
    assert_eq!(mul_div_floor(&x, &x, &d), min);
    assert_eq!(mul_div_ceil(&x, &x, &d), min);
    assert_eq!(mul_div(&x, &x, &d), min);
}

#[test]
fn checked_mul_div_result_above_i256_max_returns_none() {
    let e = Env::default();
    // The positive half of the case above, same magnitude and same denominator
    // up to sign. `2^255` has no positive representation, so the upper
    // bound rejects it one short of where the negative bound accepts.
    let x = two_pow(&e, 128);
    let d = i(&e, 2);

    assert_eq!(checked_mul_div_floor(&x, &x, &d), None);
    assert_eq!(checked_mul_div_ceil(&x, &x, &d), None);
    assert_eq!(checked_mul_div(&x, &x, &d), None);
}

#[test]
fn checked_mul_div_result_outside_range_returns_none() {
    let e = Env::default();
    // A magnitude of `3 * 2^254`, past `I256` in both directions but well
    // inside `U256`, so the summation itself succeeds and the rejection
    // comes from the sign-specific bound rather than from a checked
    // multiplication.
    let x = two_pow(&e, 128);
    let y = i(&e, 3).mul(&two_pow(&e, 126));
    let one = i(&e, 1);

    assert_eq!(x.checked_mul(&y), None);
    assert_eq!(checked_mul_div(&x, &y, &one), None);
    assert_eq!(checked_mul_div(&negate(&x), &y, &one), None);
    assert_eq!(checked_mul_div_floor(&negate(&x), &y, &one), None);
    assert_eq!(checked_mul_div_ceil(&negate(&x), &y, &one), None);
}

#[test]
fn mul_div_with_all_terms_non_zero_works() {
    let e = Env::default();
    // Every term of the decomposition live at once, including the
    // dropped-fraction term that the cases above leave at zero. With `d =
    // 5` and `a = 2^126`:
    //
    // ```text
    // x = 5a + 3, y = 5a + 4   ->   x * y = 25a^2 + 35a + 12
    // x * y / 5 = 5a^2 + 7a + 2 + 2/5
    // ```
    //
    // so `q1*q2*d`, `q1*r2`, `r1*q2` and `floor(r1*r2/d)` are `5a^2`, `4a`,
    // `3a` and 2, none of them zero.
    let a = two_pow(&e, 126);
    let five = i(&e, 5);
    let x = five.mul(&a).add(&i(&e, 3));
    let y = five.mul(&a).add(&i(&e, 4));

    assert_eq!(x.checked_mul(&y), None);

    let floor = five.mul(&a).mul(&a).add(&i(&e, 7).mul(&a)).add(&i(&e, 2));
    assert_eq!(mul_div_floor(&x, &y, &five), floor);
    assert_eq!(mul_div(&x, &y, &five), floor);
    assert_eq!(mul_div_ceil(&x, &y, &five), floor.add(&i(&e, 1)));
}

// ################## FALLBACK: TOTALITY AND FAILURE REPORTING
// ##################

#[test]
fn checked_family_never_panics_over_extremes() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    let values = [
        I256::min_value(&e),
        I256::min_value(&e).add(&i(&e, 1)),
        i(&e, -1),
        i(&e, 0),
        i(&e, 1),
        I256::max_value(&e).sub(&i(&e, 1)),
        I256::max_value(&e),
    ];

    let mut triples = 0u32;
    for x in &values {
        for y in &values {
            for d in &values {
                // Returning at all is the assertion. Zero denominators, `MIN /
                // -1` and unrepresentable results are all in
                // this grid.
                let _ = checked_mul_div_floor(x, y, d);
                let _ = checked_mul_div_ceil(x, y, d);
                let _ = checked_mul_div(x, y, d);
                triples += 1;
            }
        }
    }

    assert_eq!(triples, 343);
}

#[test]
fn prop_checked_family_never_panics() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // Unconstrained triples over the whole type, including `MIN`, which the
    // byte route reaches and `from_i128` cannot.
    proptest!(|(xb: [u8; 32], yb: [u8; 32], db: [u8; 32])| {
        let x = from_be(&e, &xb);
        let y = from_be(&e, &yb);
        let d = from_be(&e, &db);

        let _ = checked_mul_div_floor(&x, &y, &d);
        let _ = checked_mul_div_ceil(&x, &y, &d);
        let _ = checked_mul_div(&x, &y, &d);
    })
}

#[test]
fn checked_mul_div_zero_denominator_with_overflowing_product_returns_none() {
    let e = Env::default();
    // The product overflows and the denominator is zero, so the fallback runs
    // and divides by a zero magnitude. This is the case that keeps the
    // core's divisions checked rather than bare: the checked family owes a
    // `None` here, not a trap.
    let x = I256::max_value(&e);
    let y = i(&e, 2);
    let zero = i(&e, 0);

    assert_eq!(x.checked_mul(&y), None);
    assert_eq!(checked_mul_div_floor(&x, &y, &zero), None);
    assert_eq!(checked_mul_div_ceil(&x, &y, &zero), None);
    assert_eq!(checked_mul_div(&x, &y, &zero), None);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")] // SorobanFixedPointError::Overflow
fn mul_div_zero_denominator_via_fallback_panics() {
    let e = Env::default();
    // A zero denominator is reported two ways depending on operand size, a
    // documented consequence of leaving domain faults to the platform. On the
    // fast path the host's own arithmetic error surfaces, which the
    // zero-denominator tests above pin; through the fallback it is `#1500`.
    let x = I256::max_value(&e);
    let y = i(&e, 2);
    let zero = i(&e, 0);

    mul_div_floor(&x, &y, &zero);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")] // SorobanFixedPointError::Overflow
fn mul_div_ceil_unrepresentable_result_panics() {
    let e = Env::default();
    // The third unchecked entry point's own `#1500`. The two tests around this
    // one pin the code for `mul_div_floor` and `mul_div`; without this one,
    // a wrong error variant in `mul_div_ceil`'s fallback arm alone goes
    // unnoticed, because the family-agreement test only observes that an
    // abort happened, not which.
    let max = I256::max_value(&e);
    let one = i(&e, 1);

    mul_div_ceil(&max, &max, &one);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")] // SorobanFixedPointError::Overflow
fn mul_div_unrepresentable_result_panics() {
    let e = Env::default();
    // The other route to `#1500`: the decomposition runs and the true answer,
    // `MAX * MAX`, does not fit `I256`.
    let max = I256::max_value(&e);
    let one = i(&e, 1);

    mul_div(&max, &max, &one);
}

// ################## FALLBACK: FAMILY AND DISPATCHER AGREEMENT
// ##################

#[test]
fn checked_and_unchecked_families_agree_on_domain() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // The two families may report a rejection differently, `None` against a
    // panic, but they must never disagree about which inputs are
    // acceptable. Divergence is the realistic outcome of changing one
    // family and forgetting the other.
    //
    // A curated list rather than a sweep: every rejected case unwinds a real
    // host panic, and the host writes its event log on the way out.
    let two_128 = two_pow(&e, 128);
    let inputs = [
        (i(&e, 100), i(&e, 50), i(&e, 10)),         // fast path, succeeds
        (i(&e, 0), I256::max_value(&e), i(&e, -1)), // zero operand
        (I256::max_value(&e), i(&e, 2), i(&e, 2)),  // fallback, succeeds
        (I256::min_value(&e), i(&e, 3), i(&e, 3)),  // `|MIN|` has no signed form
        (two_128.clone(), two_128.add(&i(&e, 1)), two_128.clone()), // `|d|` is `2^128`
        (negate(&two_128), two_pow(&e, 127), i(&e, 1)), // result is `I256::MIN`
        (i(&e, 100), i(&e, 50), i(&e, 0)),          // zero denominator, fast path
        (I256::max_value(&e), i(&e, 2), i(&e, 0)),  // zero denominator, fallback
        (I256::min_value(&e), i(&e, 1), i(&e, -1)), // `MIN / -1`
        (I256::max_value(&e), I256::max_value(&e), i(&e, 1)), // result too large
        (two_128.clone(), two_128.clone(), two_128.add(&i(&e, 1))), // `|d|` past the domain
        (two_128.clone(), two_pow(&e, 127), i(&e, 1)), // one past `I256::MAX`
    ];

    let mut rejections = 0u32;
    for (index, (x, y, d)) in inputs.iter().enumerate() {
        let pairs: [(Option<I256>, UncheckedMulDiv); 3] = [
            (checked_mul_div_floor(x, y, d), mul_div_floor),
            (checked_mul_div_ceil(x, y, d), mul_div_ceil),
            (checked_mul_div(x, y, d), mul_div),
        ];

        for (mode, (checked, unchecked)) in pairs.into_iter().enumerate() {
            let panicked = catch_unwind(AssertUnwindSafe(|| unchecked(x, y, d))).is_err();
            assert_eq!(
                checked.is_none(),
                panicked,
                "input {index}, mode {mode}: checked and unchecked disagree"
            );
            if panicked {
                rejections += 1;
            }
        }
    }

    // Six of the twelve inputs are rejected, in all three modes. Without this
    // the test would pass on a list that happened to contain no failures at
    // all.
    assert_eq!(rejections, 18, "the rejected half of the input list stopped rejecting");
}

#[test]
fn dispatchers_match_leaf_functions_in_fallback_domain_works() {
    let e = Env::default();
    // The dispatchers stay pure delegation, so the fallback lives in the six
    // leaf functions and there is exactly one place per behaviour.
    let a = two_pow(&e, 126);
    let five = i(&e, 5);
    let x = five.mul(&a).add(&i(&e, 3));
    let y = five.mul(&a).add(&i(&e, 4));

    assert_eq!(x.checked_mul(&y), None);

    assert_eq!(
        mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Floor),
        mul_div_floor(&x, &y, &five)
    );
    assert_eq!(
        mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Ceil),
        mul_div_ceil(&x, &y, &five)
    );
    assert_eq!(
        mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Truncate),
        mul_div(&x, &y, &five)
    );
    assert_eq!(
        checked_mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Floor),
        checked_mul_div_floor(&x, &y, &five)
    );
    assert_eq!(
        checked_mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Ceil),
        checked_mul_div_ceil(&x, &y, &five)
    );
    assert_eq!(
        checked_mul_div_with_rounding(x.clone(), y.clone(), five.clone(), Rounding::Truncate),
        checked_mul_div(&x, &y, &five)
    );
}

// ################## FALLBACK: PROPERTIES ##################

#[test]
fn prop_cross_mode_consistency() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // Relates the three modes to each other, so it needs no reference
    // implementation and still catches a wrong exactness flag or a wrong sign.
    proptest!(|(xb: [u8; 32], yb: [u8; 32], db: [u8; 32], sx: bool, sy: bool, sd: bool)| {
        let x = operand(&e, &xb, sx);
        let y = operand(&e, &yb, sy);
        let d = denominator(&e, &db, sd);
        prop_assert_eq!(x.checked_mul(&y), None);
        let zero = i(&e, 0);

        let floor = mul_div_floor(&x, &y, &d);
        let ceil = mul_div_ceil(&x, &y, &d);
        let truncate = mul_div(&x, &y, &d);

        prop_assert!(floor <= truncate);
        prop_assert!(truncate <= ceil);

        let gap = ceil.sub(&floor);
        prop_assert!(gap == zero || gap == i(&e, 1));

        // Truncation follows the sign of the exact quotient, which the operand signs
        // give directly. Reading the sign off `truncate` instead would be wrong for a
        // quotient in `-1 .. 0`, where truncation lands on zero.
        if (x < zero) ^ (y < zero) ^ (d < zero) {
            prop_assert_eq!(&truncate, &ceil);
        } else {
            prop_assert_eq!(&truncate, &floor);
        }
    })
}

#[test]
fn prop_fallback_is_commutative_in_x_and_y() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // The four-term form is symmetric in `x` and `y` by construction, which was
    // a stated reason for choosing it over the cheaper three-term form.
    proptest!(|(xb: [u8; 32], yb: [u8; 32], db: [u8; 32], sx: bool, sy: bool, sd: bool)| {
        let x = operand(&e, &xb, sx);
        let y = operand(&e, &yb, sy);
        let d = denominator(&e, &db, sd);
        prop_assert_eq!(x.checked_mul(&y), None);

        prop_assert_eq!(mul_div_floor(&x, &y, &d), mul_div_floor(&y, &x, &d));
        prop_assert_eq!(mul_div_ceil(&x, &y, &d), mul_div_ceil(&y, &x, &d));
        prop_assert_eq!(mul_div(&x, &y, &d), mul_div(&y, &x, &d));
    })
}

#[test]
fn prop_negating_x_swaps_floor_and_ceil() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // `floor(-a) == -ceil(a)`, and truncation is odd. `I256::MIN` never reaches
    // a negated position: the generator caps operand magnitudes below
    // `2^152`, so negation is defined for every draw rather than filtered
    // afterwards.
    proptest!(|(xb: [u8; 32], yb: [u8; 32], db: [u8; 32], sx: bool, sy: bool, sd: bool)| {
        let x = operand(&e, &xb, sx);
        let y = operand(&e, &yb, sy);
        let d = denominator(&e, &db, sd);
        prop_assert_eq!(x.checked_mul(&y), None);
        let neg_x = negate(&x);

        prop_assert_eq!(mul_div_floor(&neg_x, &y, &d), negate(&mul_div_ceil(&x, &y, &d)));
        prop_assert_eq!(mul_div_ceil(&neg_x, &y, &d), negate(&mul_div_floor(&x, &y, &d)));
        prop_assert_eq!(mul_div(&neg_x, &y, &d), negate(&mul_div(&x, &y, &d)));
    })
}

#[test]
fn prop_negating_two_operands_preserves_result() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    // Negating two of the three operands preserves both sign and magnitude.
    proptest!(|(xb: [u8; 32], yb: [u8; 32], db: [u8; 32], sx: bool, sy: bool, sd: bool)| {
        let x = operand(&e, &xb, sx);
        let y = operand(&e, &yb, sy);
        let d = denominator(&e, &db, sd);
        prop_assert_eq!(x.checked_mul(&y), None);
        let (neg_x, neg_y, neg_d) = (negate(&x), negate(&y), negate(&d));

        prop_assert_eq!(mul_div_floor(&neg_x, &neg_y, &d), mul_div_floor(&x, &y, &d));
        prop_assert_eq!(mul_div_ceil(&neg_x, &neg_y, &d), mul_div_ceil(&x, &y, &d));
        prop_assert_eq!(mul_div(&neg_x, &neg_y, &d), mul_div(&x, &y, &d));

        prop_assert_eq!(mul_div_floor(&neg_x, &y, &neg_d), mul_div_floor(&x, &y, &d));
        prop_assert_eq!(mul_div_ceil(&neg_x, &y, &neg_d), mul_div_ceil(&x, &y, &d));
        prop_assert_eq!(mul_div(&neg_x, &y, &neg_d), mul_div(&x, &y, &d));
    })
}
