#![cfg(test)]

extern crate std;

use soroban_sdk::Env;

use crate::math::wad::*;

#[test]
fn test_from_ratio_half() {
    let e = Env::default();

    let half = Wad::from_ratio(&e, 1, 2);
    assert_eq!(half, Wad::from_integer(&e, 1) / 2);
}

#[test]
fn test_from_token_amount_less_decimals() {
    let e = Env::default();
    let amount: i128 = 1_000_000; // 1 token with 6 decimals
    let wad = Wad::from_token_amount(&e, amount, 6);
    assert_eq!(wad, Wad::from_integer(&e, 1));
}

#[test]
fn test_from_token_amount_more_decimals() {
    let e = Env::default();
    let amount: i128 = 100_000_000_000_000_000_000; // 1 token with 20 decimals
    let wad = Wad::from_token_amount(&e, amount, 20);
    assert_eq!(wad, Wad::from_integer(&e, 1));
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_from_token_amount_invalid_decimals() {
    let e = Env::default();

    let amount = 1_000_000;
    let invalid_decimals = 57u8;

    let _ = Wad::from_token_amount(&e, amount, invalid_decimals);
}

#[test]
fn test_to_token_amount_roundtrip() {
    let e = Env::default();
    let wad = Wad::from_integer(&e, 1);
    let amount_6 = wad.to_token_amount(&e, 6);
    assert_eq!(amount_6, 1_000_000);

    let amount_18 = wad.to_token_amount(&e, 18);
    assert_eq!(amount_18, WAD_SCALE);
}

#[test]
fn test_wad_mul_and_div_operators() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 3);
    let b = Wad::from_integer(&e, 2);

    let prod = a * b;
    assert_eq!(prod, Wad::from_integer(&e, 6));

    let quotient = prod / b;
    assert_eq!(quotient, a);
}

#[test]
fn test_wad_add_sub_operators() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 5);
    let b = Wad::from_integer(&e, 3);

    let sum = a + b;
    assert_eq!(sum, Wad::from_integer(&e, 8));

    let diff = a - b;
    assert_eq!(diff, Wad::from_integer(&e, 2));
}

#[test]
fn test_wad_mul_int() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 2);
    let result = a * 3;
    assert_eq!(result, Wad::from_integer(&e, 6));

    let result2 = 3 * a;
    assert_eq!(result2, Wad::from_integer(&e, 6));
}

#[test]
fn test_wad_div_int() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 6);
    let result = a / 2;
    assert_eq!(result, Wad::from_integer(&e, 3));
}

#[test]
fn test_wad_negation() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 5);
    let neg_a = -a;
    assert_eq!(neg_a, Wad::from_integer(&e, -5));
}

#[test]
fn test_price_to_wad() {
    let e = Env::default();
    let price_units: i128 = 1_000_000; // 1.0 with 6 decimals
    let wad_price = Wad::from_price(&e, price_units, 6);
    assert_eq!(wad_price, Wad::from_integer(&e, 1));
}

#[test]
fn test_wad_min_max() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let b = Wad::from_integer(&e, 2);

    assert_eq!(a.min(b), a);
    assert_eq!(a.max(b), b);
}

#[test]
fn test_wad_comparison() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let b = Wad::from_integer(&e, 2);

    assert!(a < b);
    assert!(b > a);
    assert!(a <= b);
    assert!(b >= a);
    assert_eq!(a, a);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_from_integer_overflow() {
    let e = Env::default();
    let _ = Wad::from_integer(&e, i128::MAX);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_from_ratio_overflow() {
    let e = Env::default();
    let _ = Wad::from_ratio(&e, i128::MAX, 1);
}

#[test]
fn test_from_ratio_phantom_overflow_handled() {
    let e = Env::default();
    // Large numerator that would overflow i128 when multiplied by WAD_SCALE
    // but the final result after division fits in i128
    // 10^20 * WAD_SCALE / 10^20 = WAD_SCALE (1 WAD)
    let large = 100_000_000_000_000_000_000_i128; // 10^20
    let wad = Wad::from_ratio(&e, large, large);
    assert_eq!(wad, Wad::from_integer(&e, 1));
}

#[test]
fn test_from_ratio_large_values_phantom_overflow() {
    let e = Env::default();
    // Test: 5 trillion / 2.5 trillion = 2
    // 5 * 10^12 * WAD_SCALE would overflow i128, but result (2 WAD) fits
    let num = 5_000_000_000_000_i128; // 5 trillion
    let den = 2_500_000_000_000_i128; // 2.5 trillion
    let wad = Wad::from_ratio(&e, num, den);
    assert_eq!(wad, Wad::from_integer(&e, 2));
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_from_ratio_division_by_zero() {
    let e = Env::default();
    let _ = Wad::from_ratio(&e, 100, 0);
}

#[test]
#[should_panic]
fn test_add_overflow() {
    let a = Wad::from_raw(i128::MAX);
    let b = Wad::from_raw(1);
    let _ = a + b;
}

#[test]
#[should_panic]
fn test_sub_overflow() {
    let a = Wad::from_raw(i128::MIN);
    let b = Wad::from_raw(1);
    let _ = a - b;
}

#[test]
#[should_panic]
fn test_mul_overflow() {
    let e = Env::default();
    let a = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let b = Wad::from_integer(&e, 2);
    let _ = a * b;
}

#[test]
#[should_panic]
fn test_div_by_zero() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let b = Wad::from_raw(0);
    let _ = a / b;
}

#[test]
#[should_panic]
fn test_mul_int_overflow() {
    let a = Wad::from_raw(i128::MAX);
    let _ = a * 2;
}

#[test]
#[should_panic]
fn test_div_int_by_zero() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let _ = a / 0;
}

#[test]
fn test_checked_add() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let b = Wad::from_integer(&e, 2);
    let result = a.checked_add(b);
    assert_eq!(result, Some(Wad::from_integer(&e, 3)));
}

#[test]
fn test_checked_add_overflow() {
    let a = Wad::from_raw(i128::MAX);
    let b = Wad::from_raw(1);
    let result = a.checked_add(b);
    assert_eq!(result, None);
}

#[test]
fn test_checked_div_by_zero() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 1);
    let b = Wad::from_raw(0);
    let result = a.checked_div(&e, b);
    assert_eq!(result, None);
}

#[test]
fn test_to_integer() {
    let wad = Wad::from_raw(5_500_000_000_000_000_000); // 5.5 in WAD
    assert_eq!(wad.to_integer(), 5); // truncates to 5
}

#[test]
fn test_from_token_amount_exact_18_decimals() {
    let e = Env::default();
    let amount: i128 = 1_000_000_000_000_000_000; // 1 token with 18 decimals
    let wad = Wad::from_token_amount(&e, amount, 18);
    assert_eq!(wad.raw(), amount); // no conversion needed
}

#[test]
fn test_to_token_amount_exact_18_decimals() {
    let e = Env::default();
    let wad = Wad::from_raw(1_000_000_000_000_000_000);
    let amount = wad.to_token_amount(&e, 18);
    assert_eq!(amount, 1_000_000_000_000_000_000); // no conversion needed
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_to_token_amount_overflow_high_decimals() {
    let e = Env::default();
    let wad = Wad::from_raw(i128::MAX);
    let _ = wad.to_token_amount(&e, 30); // 30 decimals requires
                                         // multiplication, will overflow
}

#[test]
fn test_raw_accessor() {
    let wad = Wad::from_raw(12345);
    assert_eq!(wad.raw(), 12345);
}

#[test]
fn test_from_raw_constructor() {
    let wad = Wad::from_raw(98765);
    assert_eq!(wad.raw(), 98765);
}

#[test]
fn test_min_returns_other() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 5);
    let b = Wad::from_integer(&e, 3);
    assert_eq!(a.min(b), b); // returns other (b) since b < a
}

#[test]
fn test_max_returns_other() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 3);
    let b = Wad::from_integer(&e, 5);
    assert_eq!(a.max(b), b); // returns other (b) since b > a
}

#[test]
fn test_max_returns_self() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 5);
    let b = Wad::from_integer(&e, 3);
    assert_eq!(a.max(b), a); // returns self (a) since a > b
}

#[test]
fn test_checked_sub_success() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 5);
    let b = Wad::from_integer(&e, 3);
    let result = a.checked_sub(b);
    assert_eq!(result, Some(Wad::from_integer(&e, 2)));
}

#[test]
fn test_checked_sub_overflow() {
    let a = Wad::from_raw(i128::MIN);
    let b = Wad::from_raw(1);
    let result = a.checked_sub(b);
    assert_eq!(result, None);
}

#[test]
fn test_checked_mul_success() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 2);
    let b = Wad::from_integer(&e, 3);
    let result = a.checked_mul(&e, b);
    assert_eq!(result, Some(Wad::from_integer(&e, 6)));
}

#[test]
fn test_checked_mul_phantom_overflow_handled() {
    let e = Env::default();
    // With phantom overflow handling, intermediate overflow is OK
    // if final result fits
    let sixteen = Wad::from_integer(&e, 16);
    let result = sixteen.checked_mul(&e, sixteen);
    assert_eq!(result, Some(Wad::from_integer(&e, 256)));
}

#[test]
fn test_checked_mul_true_overflow() {
    let e = Env::default();
    // True overflow: even with i256, result doesn't fit in i128
    let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let result = large.checked_mul(&e, large);
    assert_eq!(result, None);
}

#[test]
fn test_checked_div_success() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 6);
    let b = Wad::from_integer(&e, 2);
    let result = a.checked_div(&e, b);
    assert_eq!(result, Some(Wad::from_integer(&e, 3)));
}

#[test]
fn test_checked_div_overflow() {
    let e = Env::default();
    let a = Wad::from_raw(i128::MAX);
    let b = Wad::from_raw(1);
    let result = a.checked_div(&e, b); // MAX * WAD_SCALE will overflow even with I256
    assert_eq!(result, None);
}

#[test]
fn test_checked_div_phantom_overflow_handled() {
    let e = Env::default();
    // 5000 WAD / 5000 WAD = 1 WAD
    // The intermediate calculation (5000 * WAD_SCALE * WAD_SCALE) would overflow
    // i128 but the final result (1 WAD) fits, so phantom overflow should be
    // handled
    let a = Wad::from_integer(&e, 5_000);
    let b = Wad::from_integer(&e, 5_000);
    let result = a.checked_div(&e, b);
    assert_eq!(result, Some(Wad::from_integer(&e, 1)));
}

#[test]
fn test_checked_div_large_values_phantom_overflow() {
    let e = Env::default();
    // Test with even larger values that would definitely overflow i128
    // but produce a valid result after division
    let large_value = 1_000_000_000_000_i128; // 1 trillion
    let a = Wad::from_integer(&e, large_value);
    let b = Wad::from_integer(&e, large_value);
    let result = a.checked_div(&e, b);
    // 1 trillion / 1 trillion = 1
    assert_eq!(result, Some(Wad::from_integer(&e, 1)));
}

#[test]
fn test_checked_mul_int_success() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 2);
    let result = a.checked_mul_int(5);
    assert_eq!(result, Some(Wad::from_integer(&e, 10)));
}

#[test]
fn test_checked_mul_int_overflow() {
    let a = Wad::from_raw(i128::MAX);
    let result = a.checked_mul_int(2);
    assert_eq!(result, None);
}

#[test]
fn test_checked_div_int_success() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 10);
    let result = a.checked_div_int(2);
    assert_eq!(result, Some(Wad::from_integer(&e, 5)));
}

#[test]
fn test_checked_div_int_by_zero() {
    let e = Env::default();
    let a = Wad::from_integer(&e, 10);
    let result = a.checked_div_int(0);
    assert_eq!(result, None);
}

#[test]
fn test_neg_positive() {
    let e = Env::default();
    let positive = Wad::from_integer(&e, 5);
    let negative = -positive;
    assert_eq!(negative, Wad::from_integer(&e, -5));
}

#[test]
fn test_neg_negative() {
    let e = Env::default();
    let negative = Wad::from_integer(&e, -5);
    let positive = -negative;
    assert_eq!(positive, Wad::from_integer(&e, 5));
}

#[test]
fn test_neg_zero() {
    let zero = Wad::from_raw(0);
    let neg_zero = -zero;
    assert_eq!(neg_zero, Wad::from_raw(0));
}

#[test]
fn test_abs_positive() {
    let e = Env::default();
    let positive = Wad::from_integer(&e, 5);
    assert_eq!(positive.abs(), Wad::from_integer(&e, 5));
}

#[test]
fn test_abs_negative() {
    let e = Env::default();
    let negative = Wad::from_integer(&e, -5);
    assert_eq!(negative.abs(), Wad::from_integer(&e, 5));
}

#[test]
fn test_abs_zero() {
    let zero = Wad::from_raw(0);
    assert_eq!(zero.abs(), Wad::from_raw(0));
}

#[test]
fn test_abs_with_fractional() {
    let negative_fraction = Wad::from_raw(-500_000_000_000_000_000); // -0.5
    let positive_fraction = Wad::from_raw(500_000_000_000_000_000); // 0.5
    assert_eq!(negative_fraction.abs(), positive_fraction);
}

// ################## POW TESTS ##################

#[test]
fn test_pow_zero_exponent() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 42);
    let result = base.pow(&e, 0);
    assert_eq!(result, Wad::from_integer(&e, 1)); // x^0 = 1
}

#[test]
fn test_pow_zero_base_zero_exponent() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.pow(&e, 0);
    assert_eq!(result, Wad::from_integer(&e, 1)); // 0^0 = 1 (by convention)
}

#[test]
fn test_pow_one_exponent() {
    let e = Env::default();
    let base = Wad::from_ratio(&e, 355, 113); // π approximation
    let result = base.pow(&e, 1);
    assert_eq!(result, base); // x^1 = x
}

#[test]
fn test_pow_zero_base() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.pow(&e, 5);
    assert_eq!(result, Wad::from_raw(0)); // 0^n = 0 for n > 0
}

#[test]
fn test_pow_one_base() {
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    let result = one.pow(&e, 100);
    assert_eq!(result, Wad::from_integer(&e, 1)); // 1^n = 1
}

#[test]
fn test_pow_integer_base_small_exponent() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.pow(&e, 10);
    assert_eq!(result, Wad::from_integer(&e, 1024)); // 2^10 = 1024
}

#[test]
fn test_pow_integer_base_quadratic() {
    let e = Env::default();
    let five = Wad::from_integer(&e, 5);
    let result = five.pow(&e, 2);
    assert_eq!(result, Wad::from_integer(&e, 25)); // 5^2 = 25
}

#[test]
fn test_pow_integer_base_cubic() {
    let e = Env::default();
    let three = Wad::from_integer(&e, 3);
    let result = three.pow(&e, 3);
    assert_eq!(result, Wad::from_integer(&e, 27)); // 3^3 = 27
}

#[test]
fn test_pow_compound_interest() {
    let e = Env::default();
    // 5% annual rate: 1.05
    let rate = Wad::from_ratio(&e, 105, 100);
    let result = rate.pow(&e, 10);

    // (1.05)^10 ≈ 1.62889462677744140625
    // In WAD: 1_628_894_626_777_441_406 (with truncation)
    let expected = Wad::from_raw(1_628_894_626_777_441_406);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_fractional_base_squared() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2); // 0.5
    let result = half.pow(&e, 2);

    // 0.5^2 = 0.25
    let expected = Wad::from_ratio(&e, 1, 4);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_fractional_base_cubed() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2); // 0.5
    let result = half.pow(&e, 3);

    // 0.5^3 = 0.125
    let expected = Wad::from_ratio(&e, 1, 8);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_fractional_less_than_one() {
    let e = Env::default();
    // 0.95
    let decay = Wad::from_ratio(&e, 95, 100);
    let result = decay.pow(&e, 10);

    // (0.95)^10 ≈ 0.59873693923837890625
    // In WAD: 598_736_939_238_378_906 (with truncation)
    let expected = Wad::from_raw(598_736_939_238_378_906);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_negative_base_even_exponent() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    let result = neg_two.pow(&e, 4);
    assert_eq!(result, Wad::from_integer(&e, 16)); // (-2)^4 = 16
}

#[test]
fn test_pow_negative_base_odd_exponent() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    let result = neg_two.pow(&e, 3);
    assert_eq!(result, Wad::from_integer(&e, -8)); // (-2)^3 = -8
}

#[test]
fn test_pow_negative_fractional_even() {
    let e = Env::default();
    let neg_half = Wad::from_ratio(&e, -1, 2); // -0.5
    let result = neg_half.pow(&e, 2);

    // (-0.5)^2 = 0.25
    let expected = Wad::from_ratio(&e, 1, 4);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_negative_fractional_odd() {
    let e = Env::default();
    let neg_half = Wad::from_ratio(&e, -1, 2); // -0.5
    let result = neg_half.pow(&e, 3);

    // (-0.5)^3 = -0.125
    let expected = Wad::from_ratio(&e, -1, 8);
    assert_eq!(result, expected);
}

#[test]
fn test_pow_precision_decay() {
    let e = Env::default();
    // Each power operation involves truncation
    let base = Wad::from_ratio(&e, 999, 1000); // 0.999
    let result = base.pow(&e, 100);

    // (0.999)^100 =
    // actual          -> 0.9047921471137089XX
    // expected_approx -> 0.904792147113709XXX

    // Expect some truncation in lower digits
    let expected_approx = Wad::from_raw(904_792_147_113_709_024);
    assert_eq!(result, expected_approx);
}

#[test]
fn test_pow_large_integer_base() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 10);
    let result = base.pow(&e, 5);
    assert_eq!(result, Wad::from_integer(&e, 100_000)); // 10^5 = 100,000
}

#[test]
fn test_pow_truncation_behavior() {
    let e = Env::default();
    // Use a base that will produce fractional intermediate results
    let base = Wad::from_raw(1_414_213_562_373_095_048); // √2 ≈ 1.414213562373095048
    let result = base.pow(&e, 2);

    // (√2)^2 should be very close to 2, but truncation may cause slight deviation
    let two = Wad::from_integer(&e, 2);
    let diff = (result.raw() - two.raw()).abs();

    // Allow for minimal truncation error (within 10 units out of 10^18)
    assert!(diff <= 10);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_pow_overflow_large_base() {
    let e = Env::default();
    // Base that's too large
    let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let _ = large.pow(&e, 2); // Will overflow
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_pow_overflow_large_exponent() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let _ = two.pow(&e, 128); // 2^128 overflows i128
}

// ################## CHECKED_POW TESTS ##################

#[test]
fn test_checked_pow_success() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.checked_pow(&e, 10);
    assert_eq!(result, Some(Wad::from_integer(&e, 1024)));
}

#[test]
fn test_checked_pow_overflow_returns_none() {
    let e = Env::default();
    let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let result = large.checked_pow(&e, 2);
    assert_eq!(result, None); // Overflow returns None instead of panicking
}

#[test]
fn test_checked_pow_zero_exponent() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 42);
    let result = base.checked_pow(&e, 0);
    assert_eq!(result, Some(Wad::from_integer(&e, 1)));
}

#[test]
fn test_checked_pow_one_exponent() {
    let e = Env::default();
    let base = Wad::from_ratio(&e, 355, 113);
    let result = base.checked_pow(&e, 1);
    assert_eq!(result, Some(base));
}

#[test]
fn test_checked_pow_zero_base() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.checked_pow(&e, 5);
    assert_eq!(result, Some(Wad::from_raw(0)));
}

#[test]
fn test_checked_pow_compound_interest() {
    let e = Env::default();
    let rate = Wad::from_ratio(&e, 105, 100); // 1.05
    let result = rate.checked_pow(&e, 10);

    let expected = Wad::from_raw(1_628_894_626_777_441_406);
    assert_eq!(result, Some(expected));
}

#[test]
fn test_checked_pow_large_exponent_overflow() {
    /*
    2^n × 10^18 ≤ i128::MAX
    2^n × 2^59.79 ≤ 2^127
    2^(n + 59.79) ≤ 2^127
    n + 59.79 ≤ 127
    n ≤ 67.21
    */
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.checked_pow(&e, 68);
    assert_eq!(result, None);
}

#[test]
fn test_checked_pow_fractional_base() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2);
    let result = half.checked_pow(&e, 3);
    assert_eq!(result, Some(Wad::from_ratio(&e, 1, 8)));
}
