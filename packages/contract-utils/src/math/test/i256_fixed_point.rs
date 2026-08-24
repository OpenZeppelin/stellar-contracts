#![cfg(test)]

extern crate std;

use soroban_sdk::{Env, I256};

use crate::math::{
    i256_fixed_point::{
        checked_mul_div, checked_mul_div_ceil, checked_mul_div_floor,
        checked_mul_div_with_rounding, mul_div, mul_div_ceil, mul_div_floor, mul_div_with_rounding,
    },
    Rounding,
};

#[test]
#[should_panic]
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
    // x * 10^39 overflows the I256 intermediate (max ~5.8e76), so this exercises
    // the fallback. The quotient is 10^21 * x, which fits, so it is recovered
    // rather than failing.
    //
    // 10^39 is the multiplier we need, but it exceeds i128::MAX (~1.7e38), so it
    // cannot be written as `10i128.pow(39)`. It is built in I256 space instead.
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
    // Same input as the floor case. The division is exact, so ceil returns the same
    // value.
    //
    // 10^39 exceeds i128::MAX (~1.7e38), so it is built in I256 space rather than
    // as `10i128.pow(39)`.
    let y: I256 = I256::from_i128(&env, 10i128.pow(20)).mul(&I256::from_i128(&env, 10i128.pow(19)));
    let denominator: I256 = I256::from_i128(&env, 10i128.pow(18));

    let result = mul_div_ceil(&x, &y, &denominator);

    assert_eq!(result, x.mul(&I256::from_i128(&env, 10i128.pow(21))));
}

#[test]
#[should_panic]
fn test_mul_div_floor_zero_denominator() {
    let env = Env::default();
    let x: I256 = I256::from_i128(&env, 100);
    let y: I256 = I256::from_i128(&env, 50);
    let denominator: I256 = I256::from_i128(&env, 0);

    mul_div_floor(&x, &y, &denominator);
}

#[test]
#[should_panic]
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
    // x * y = I256::MAX * 2 overflows I256, but the quotient is exactly I256::MAX.
    // The remainders are both zero, so the decomposition recovers the answer
    // and the checked variants return `Some` where they used to return `None`.
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
#[should_panic]
fn test_mul_div_min_by_negative_one_panics_untyped() {
    let env = Env::default();
    // `I256::MIN / -1` is `2^255`, which has no `I256` representation.
    // `checked_mul` succeeds and the overflow happens inside the unchecked
    // division, so this is the one failure the module reports as a native
    // arithmetic panic rather than a contract error code. Pinned deliberately
    // rather than fixed: the i128 sibling has the identical trap, and routing
    // the fast path through the checked helper would re-price every non-overflowing
    // call.
    let x: I256 = I256::min_value(&env);
    let y: I256 = I256::from_i128(&env, 1);
    let denominator: I256 = I256::from_i128(&env, -1);

    mul_div_floor(&x, &y, &denominator);
}

#[test]
fn test_checked_mul_div_min_by_negative_one_returns_none() {
    let env = Env::default();
    // The checked counterpart of the case above. Both fail, which is what keeps the
    // two families in agreement on their domain; only the reporting differs.
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
