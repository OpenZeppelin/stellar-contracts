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
    // 10^21 * WAD_SCALE (10^18) = 10^39 which overflows i128 (max ~1.7*10^38)
    // but 10^21 / 10^21 = 1, so final result (1 WAD) fits
    let large = 1_000_000_000_000_000_000_000_i128; // 10^21
    let wad = Wad::from_ratio(&e, large, large);
    assert_eq!(wad, Wad::from_integer(&e, 1));
}

#[test]
fn test_from_ratio_large_values_phantom_overflow() {
    let e = Env::default();
    // 10^22 * WAD_SCALE (10^18) = 10^40 which overflows i128
    // but 10^22 / (5 * 10^21) = 2, so final result (2 WAD) fits
    let num = 10_000_000_000_000_000_000_000_i128; // 10^22
    let den = 5_000_000_000_000_000_000_000_i128; // 5 * 10^21
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
fn test_powi_zero_exponent() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 42);
    let result = base.powi(&e, 0);
    assert_eq!(result, Wad::from_integer(&e, 1)); // x^0 = 1
}

#[test]
fn test_powi_zero_base_zero_exponent() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.powi(&e, 0);
    assert_eq!(result, Wad::from_integer(&e, 1)); // 0^0 = 1 (by convention)
}

#[test]
fn test_powi_one_exponent() {
    let e = Env::default();
    let base = Wad::from_ratio(&e, 355, 113); // π approximation
    let result = base.powi(&e, 1);
    assert_eq!(result, base); // x^1 = x
}

#[test]
fn test_powi_zero_base() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.powi(&e, 5);
    assert_eq!(result, Wad::from_raw(0)); // 0^n = 0 for n > 0
}

#[test]
fn test_powi_one_base() {
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    let result = one.powi(&e, 100);
    assert_eq!(result, Wad::from_integer(&e, 1)); // 1^n = 1
}

#[test]
fn test_powi_integer_base_small_exponent() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.powi(&e, 10);
    assert_eq!(result, Wad::from_integer(&e, 1024)); // 2^10 = 1024
}

#[test]
fn test_powi_integer_base_quadratic() {
    let e = Env::default();
    let five = Wad::from_integer(&e, 5);
    let result = five.powi(&e, 2);
    assert_eq!(result, Wad::from_integer(&e, 25)); // 5^2 = 25
}

#[test]
fn test_powi_integer_base_cubic() {
    let e = Env::default();
    let three = Wad::from_integer(&e, 3);
    let result = three.powi(&e, 3);
    assert_eq!(result, Wad::from_integer(&e, 27)); // 3^3 = 27
}

#[test]
fn test_powi_compound_interest() {
    let e = Env::default();
    // 5% annual rate: 1.05
    let rate = Wad::from_ratio(&e, 105, 100);
    let result = rate.powi(&e, 10);

    // (1.05)^10 ≈ 1.62889462677744140625
    // In WAD: 1_628_894_626_777_441_406 (with truncation)
    let expected = Wad::from_raw(1_628_894_626_777_441_406);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_fractional_base_squared() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2); // 0.5
    let result = half.powi(&e, 2);

    // 0.5^2 = 0.25
    let expected = Wad::from_ratio(&e, 1, 4);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_fractional_base_cubed() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2); // 0.5
    let result = half.powi(&e, 3);

    // 0.5^3 = 0.125
    let expected = Wad::from_ratio(&e, 1, 8);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_fractional_less_than_one() {
    let e = Env::default();
    // 0.95
    let decay = Wad::from_ratio(&e, 95, 100);
    let result = decay.powi(&e, 10);

    // (0.95)^10 ≈ 0.59873693923837890625
    // In WAD: 598_736_939_238_378_906 (with truncation)
    let expected = Wad::from_raw(598_736_939_238_378_906);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_negative_base_even_exponent() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    let result = neg_two.powi(&e, 4);
    assert_eq!(result, Wad::from_integer(&e, 16)); // (-2)^4 = 16
}

#[test]
fn test_powi_negative_base_odd_exponent() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    let result = neg_two.powi(&e, 3);
    assert_eq!(result, Wad::from_integer(&e, -8)); // (-2)^3 = -8
}

#[test]
fn test_powi_negative_fractional_even() {
    let e = Env::default();
    let neg_half = Wad::from_ratio(&e, -1, 2); // -0.5
    let result = neg_half.powi(&e, 2);

    // (-0.5)^2 = 0.25
    let expected = Wad::from_ratio(&e, 1, 4);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_negative_fractional_odd() {
    let e = Env::default();
    let neg_half = Wad::from_ratio(&e, -1, 2); // -0.5
    let result = neg_half.powi(&e, 3);

    // (-0.5)^3 = -0.125
    let expected = Wad::from_ratio(&e, -1, 8);
    assert_eq!(result, expected);
}

#[test]
fn test_powi_precision_decay() {
    let e = Env::default();
    // Each power operation involves truncation
    let base = Wad::from_ratio(&e, 999, 1000); // 0.999
    let result = base.powi(&e, 100);

    // (0.999)^100 =
    // actual          -> 0.9047921471137089XX
    // expected_approx -> 0.904792147113709XXX

    // Expect some truncation in lower digits
    let expected_approx = Wad::from_raw(904_792_147_113_709_024);
    assert_eq!(result, expected_approx);
}

#[test]
fn test_powi_large_integer_base() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 10);
    let result = base.powi(&e, 5);
    assert_eq!(result, Wad::from_integer(&e, 100_000)); // 10^5 = 100,000
}

#[test]
fn test_powi_truncation_behavior() {
    let e = Env::default();
    // Use a base that will produce fractional intermediate results
    let base = Wad::from_raw(1_414_213_562_373_095_048); // √2 ≈ 1.414213562373095048
    let result = base.powi(&e, 2);

    // (√2)^2 should be very close to 2, but truncation may cause slight deviation
    let two = Wad::from_integer(&e, 2);
    let diff = (result.raw() - two.raw()).abs();

    // Allow for minimal truncation error (within 10 units out of 10^18)
    assert!(diff <= 10);
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_powi_overflow_large_base() {
    let e = Env::default();
    // Base that's too large
    let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let _ = large.powi(&e, 2); // Will overflow
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_powi_overflow_large_exponent() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let _ = two.powi(&e, 128); // 2^128 overflows i128
}

// ################## CHECKED_POW TESTS ##################

#[test]
fn test_checked_powi_success() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.checked_powi(&e, 10);
    assert_eq!(result, Some(Wad::from_integer(&e, 1024)));
}

#[test]
fn test_checked_powi_overflow_returns_none() {
    let e = Env::default();
    let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    let result = large.checked_powi(&e, 2);
    assert_eq!(result, None); // Overflow returns None instead of panicking
}

#[test]
fn test_checked_powi_zero_exponent() {
    let e = Env::default();
    let base = Wad::from_integer(&e, 42);
    let result = base.checked_powi(&e, 0);
    assert_eq!(result, Some(Wad::from_integer(&e, 1)));
}

#[test]
fn test_checked_powi_one_exponent() {
    let e = Env::default();
    let base = Wad::from_ratio(&e, 355, 113);
    let result = base.checked_powi(&e, 1);
    assert_eq!(result, Some(base));
}

#[test]
fn test_checked_powi_zero_base() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let result = zero.checked_powi(&e, 5);
    assert_eq!(result, Some(Wad::from_raw(0)));
}

#[test]
fn test_checked_powi_compound_interest() {
    let e = Env::default();
    let rate = Wad::from_ratio(&e, 105, 100); // 1.05
    let result = rate.checked_powi(&e, 10);

    let expected = Wad::from_raw(1_628_894_626_777_441_406);
    assert_eq!(result, Some(expected));
}

#[test]
fn test_checked_powi_large_exponent_overflow() {
    /*
    2^n × 10^18 ≤ i128::MAX
    2^n × 2^59.79 ≤ 2^127
    2^(n + 59.79) ≤ 2^127
    n + 59.79 ≤ 127
    n ≤ 67.21
    */
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.checked_powi(&e, 68);
    assert_eq!(result, None);
}

#[test]
fn test_checked_powi_fractional_base() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2);
    let result = half.checked_powi(&e, 3);
    assert_eq!(result, Some(Wad::from_ratio(&e, 1, 8)));
}

// ################## LN TESTS ##################

/// Helper: assert two Wad values are within `tol` (tolerance) raw units of each
/// other.
fn assert_close(actual: Wad, expected: Wad, tol: i128, label: &str) {
    let diff = (actual.raw() - expected.raw()).abs();
    assert!(
        diff <= tol,
        "{label}: actual={} expected={} diff={} tol={}",
        actual.raw(),
        expected.raw(),
        diff,
        tol
    );
}

#[test]
fn test_ln_one() {
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    // ln(1) = 0 exactly
    assert_eq!(one.ln(&e), Wad::from_raw(0));
}

#[test]
fn test_ln_e() {
    let e = Env::default();
    // e ≈ 2.718281828459045235
    let e_wad = Wad::from_raw(2_718_281_828_459_045_235);
    let result = e_wad.ln(&e);
    // ln(e) = 1
    assert_close(result, Wad::from_integer(&e, 1), 5, "ln(e)");
}

#[test]
fn test_ln_two() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.ln(&e);
    // ln(2) ≈ 0.693147180559945309
    let expected = Wad::from_raw(693_147_180_559_945_309);
    assert_close(result, expected, 5, "ln(2)");
}

#[test]
fn test_ln_ten() {
    let e = Env::default();
    let ten = Wad::from_integer(&e, 10);
    let result = ten.ln(&e);
    // ln(10) ≈ 2.302585092994045684
    let expected = Wad::from_raw(2_302_585_092_994_045_684);
    assert_close(result, expected, 5, "ln(10)");
}

#[test]
fn test_ln_half() {
    let e = Env::default();
    let half = Wad::from_ratio(&e, 1, 2);
    let result = half.ln(&e);
    // ln(0.5) = -ln(2) ≈ -0.693147180559945309
    let expected = Wad::from_raw(-693_147_180_559_945_309);
    assert_close(result, expected, 5, "ln(0.5)");
}

#[test]
fn test_ln_large() {
    let e = Env::default();
    // ln(1e10) = 10 * ln(10) ≈ 23.025850929940456840
    let large = Wad::from_integer(&e, 10_000_000_000);
    let result = large.ln(&e);
    let expected = Wad::from_raw(23_025_850_929_940_456_840);
    assert_close(result, expected, 50, "ln(1e10)");
}

#[test]
fn test_ln_very_large_triggers_shr_branch() {
    let e = Env::default();
    // Raw WAD value 10^30 ≈ 2^99.66, so ilog2 = 99 and k = 3 > 0,
    // which exercises the right-shift normalization branch in ln_wad.
    // ln(1e12) = 12 * ln(10) ≈ 27.631021115928548208
    let large = Wad::from_integer(&e, 1_000_000_000_000);
    let result = large.ln(&e);
    let expected = Wad::from_raw(27_631_021_115_928_548_208);
    assert_close(result, expected, 100, "ln(1e12)");
}

#[test]
#[should_panic(expected = "Error(Contract, #1502)")]
fn test_ln_zero_panics() {
    let e = Env::default();
    let _ = Wad::from_raw(0).ln(&e);
}

#[test]
#[should_panic(expected = "Error(Contract, #1502)")]
fn test_ln_negative_panics() {
    let e = Env::default();
    let _ = Wad::from_integer(&e, -5).ln(&e);
}

#[test]
fn test_checked_ln_zero_returns_none() {
    let e = Env::default();
    assert_eq!(Wad::from_raw(0).checked_ln(&e), None);
}

#[test]
fn test_checked_ln_negative_returns_none() {
    let e = Env::default();
    assert_eq!(Wad::from_integer(&e, -1).checked_ln(&e), None);
}

#[test]
fn test_checked_ln_positive() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let result = two.checked_ln(&e).unwrap();
    let expected = Wad::from_raw(693_147_180_559_945_309);
    assert_close(result, expected, 5, "checked_ln(2)");
}

// ################## EXP TESTS ##################

#[test]
fn test_exp_zero() {
    let e = Env::default();
    let result = Wad::from_raw(0).exp(&e);
    assert_eq!(result, Wad::from_integer(&e, 1));
}

#[test]
fn test_exp_one() {
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    let result = one.exp(&e);
    // e ≈ 2.718281828459045235
    let expected = Wad::from_raw(2_718_281_828_459_045_235);
    assert_close(result, expected, 5, "exp(1)");
}

#[test]
fn test_exp_negative_one() {
    let e = Env::default();
    let neg_one = Wad::from_integer(&e, -1);
    let result = neg_one.exp(&e);
    // 1/e ≈ 0.367879441171442322
    let expected = Wad::from_raw(367_879_441_171_442_322);
    assert_close(result, expected, 5, "exp(-1)");
}

#[test]
fn test_exp_ln_two_is_two() {
    let e = Env::default();
    // ln(2) ≈ 0.693147180559945309 → exp(ln(2)) = 2
    let ln2 = Wad::from_raw(693_147_180_559_945_309);
    let result = ln2.exp(&e);
    assert_close(result, Wad::from_integer(&e, 2), 5, "exp(ln(2))");
}

#[test]
fn test_exp_underflow_returns_zero() {
    let e = Env::default();
    // Below -42.139... — Solmate convention: rounds to 0.
    let very_negative = Wad::from_integer(&e, -100);
    assert_eq!(very_negative.exp(&e), Wad::from_raw(0));
}

#[test]
fn test_exp_at_lower_bound_returns_zero() {
    let e = Env::default();
    let bound = Wad::from_raw(-42_139_678_854_452_767_551);
    assert_eq!(bound.exp(&e), Wad::from_raw(0));
}

#[test]
#[should_panic(expected = "Error(Contract, #1500)")]
fn test_exp_overflow_panics() {
    let e = Env::default();
    // Just above the upper bound (135.305...).
    let too_large = Wad::from_raw(135_305_999_368_893_231_589);
    let _ = too_large.exp(&e);
}

#[test]
fn test_checked_exp_overflow_returns_none() {
    let e = Env::default();
    let too_large = Wad::from_integer(&e, 200);
    assert_eq!(too_large.checked_exp(&e), None);
}

#[test]
fn test_checked_exp_underflow_returns_zero() {
    // Underflow is not an error — it's a defined return of 0.
    let e = Env::default();
    let very_negative = Wad::from_integer(&e, -100);
    assert_eq!(very_negative.checked_exp(&e), Some(Wad::from_raw(0)));
}

#[test]
fn test_exp_ln_roundtrip() {
    let e = Env::default();
    // exp(ln(x)) ≈ x for various positive x. Tolerance is per-magnitude
    // (~1e-15 relative): for large x, raw error scales with x.
    for raw_x in [
        500_000_000_000_000_000i128,   // 0.5
        1_000_000_000_000_000_000,     // 1
        2_500_000_000_000_000_000,     // 2.5
        100_000_000_000_000_000_000,   // 100
        1_000_000_000_000_000_000_000, // 1000
    ] {
        let x = Wad::from_raw(raw_x);
        let recovered = x.ln(&e).exp(&e);
        // ~1e-15 relative tolerance: tol = max(10, raw_x / 1e15)
        let tol = (raw_x / 1_000_000_000_000_000).max(10);
        assert_close(recovered, x, tol, "exp(ln(x)) roundtrip");
    }
}

// ################## POW_WAD TESTS ##################

#[test]
fn test_powf_y_zero() {
    let e = Env::default();
    let x = Wad::from_integer(&e, 42);
    assert_eq!(x.powf(&e, Wad::from_raw(0)), Wad::from_integer(&e, 1));
}

#[test]
fn test_powf_zero_zero() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    // 0^0 = 1 (matches existing pow convention)
    assert_eq!(zero.powf(&e, zero), Wad::from_integer(&e, 1));
}

#[test]
fn test_powf_zero_positive_y() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let y = Wad::from_integer(&e, 5);
    assert_eq!(zero.powf(&e, y), Wad::from_raw(0));
}

#[test]
#[should_panic(expected = "Error(Contract, #1501)")]
fn test_powf_zero_negative_y_panics() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let neg_y = Wad::from_integer(&e, -1);
    let _ = zero.powf(&e, neg_y);
}

#[test]
fn test_powf_one_base() {
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    let weird_y = Wad::from_ratio(&e, 355, 113); // π-ish
    assert_eq!(one.powf(&e, weird_y), one);
}

#[test]
fn test_powf_y_one() {
    let e = Env::default();
    let x = Wad::from_ratio(&e, 7, 13);
    let one = Wad::from_integer(&e, 1);
    assert_eq!(x.powf(&e, one), x);
}

#[test]
fn test_powf_integer_exponent_matches_powi() {
    // Integer-y fast path should produce identical results to pow(u32).
    let e = Env::default();
    for &(raw_x, n) in &[
        (2_000_000_000_000_000_000i128, 10u32), // 2^10
        (1_050_000_000_000_000_000, 20),        // 1.05^20
        (3_000_000_000_000_000_000, 5),         // 3^5
    ] {
        let x = Wad::from_raw(raw_x);
        let y_wad = Wad::from_integer(&e, n as i128);
        assert_eq!(x.powf(&e, y_wad), x.powi(&e, n));
    }
}

#[test]
fn test_powf_negative_base_integer_exponent() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    // (-2)^3 = -8 — handled by integer fast path.
    assert_eq!(neg_two.powf(&e, Wad::from_integer(&e, 3)), Wad::from_integer(&e, -8));
    // (-2)^4 = 16
    assert_eq!(neg_two.powf(&e, Wad::from_integer(&e, 4)), Wad::from_integer(&e, 16));
}

#[test]
fn test_powf_sqrt_two() {
    let e = Env::default();
    // 2^0.5 = √2 ≈ 1.414213562373095048
    let two = Wad::from_integer(&e, 2);
    let half = Wad::from_ratio(&e, 1, 2);
    let result = two.powf(&e, half);
    let expected = Wad::from_raw(1_414_213_562_373_095_048);
    assert_close(result, expected, 10, "2^0.5");
}

#[test]
fn test_powf_two_to_the_2_5() {
    let e = Env::default();
    // 2^2.5 = 4 * √2 ≈ 5.656854249492380195
    let two = Wad::from_integer(&e, 2);
    let exp = Wad::from_raw(2_500_000_000_000_000_000); // 2.5
    let result = two.powf(&e, exp);
    let expected = Wad::from_raw(5_656_854_249_492_380_195);
    assert_close(result, expected, 50, "2^2.5");
}

#[test]
fn test_powf_compound_interest_fractional() {
    let e = Env::default();
    // (1.05)^2.5 ≈ 1.126162419641586023
    let rate = Wad::from_ratio(&e, 105, 100);
    let years = Wad::from_raw(2_500_000_000_000_000_000); // 2.5
    let result = rate.powf(&e, years);
    // Truncated 1.129726321947045721750119514527445981979...
    let expected = Wad::from_raw(1_129_726_321_947_045_721);
    assert_close(result, expected, 10, "1.05^2.5");
}

#[test]
fn test_powf_negative_exponent() {
    let e = Env::default();
    // 2^(-1) = 0.5 — falls through to general path (negative non-integer-flagged y)
    let two = Wad::from_integer(&e, 2);
    let neg_one = Wad::from_integer(&e, -1);
    let result = two.powf(&e, neg_one);
    let expected = Wad::from_ratio(&e, 1, 2);
    assert_close(result, expected, 10, "2^-1");
}

#[test]
fn test_powf_negative_fractional_exponent() {
    let e = Env::default();
    // 2^(-0.5) = 1/√2 ≈ 0.707106781186547524
    let two = Wad::from_integer(&e, 2);
    let neg_half = Wad::from_ratio(&e, -1, 2);
    let result = two.powf(&e, neg_half);
    let expected = Wad::from_raw(707_106_781_186_547_524);
    assert_close(result, expected, 10, "2^-0.5");
}

#[test]
#[should_panic(expected = "Error(Contract, #1502)")]
fn test_powf_negative_base_non_integer_panics() {
    let e = Env::default();
    // (-2)^0.5 — undefined in real numbers.
    let neg_two = Wad::from_integer(&e, -2);
    let half = Wad::from_ratio(&e, 1, 2);
    let _ = neg_two.powf(&e, half);
}

#[test]
fn test_checked_powf_overflow_returns_none() {
    let e = Env::default();
    // 1000^1000 overflows wildly.
    let big = Wad::from_integer(&e, 1000);
    let result = big.checked_powf(&e, big);
    assert_eq!(result, None);
}

#[test]
fn test_checked_powf_negative_base_returns_none() {
    let e = Env::default();
    let neg_two = Wad::from_integer(&e, -2);
    let half = Wad::from_ratio(&e, 1, 2);
    assert_eq!(neg_two.checked_powf(&e, half), None);
}

#[test]
fn test_checked_powf_zero_negative_returns_none() {
    let e = Env::default();
    let zero = Wad::from_raw(0);
    let neg_one = Wad::from_integer(&e, -1);
    assert_eq!(zero.checked_powf(&e, neg_one), None);
}

#[test]
fn test_checked_powf_success() {
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let half = Wad::from_ratio(&e, 1, 2);
    let result = two.checked_powf(&e, half).unwrap();
    let expected = Wad::from_raw(1_414_213_562_373_095_048);
    assert_close(result, expected, 10, "checked_powf(2, 0.5)");
}

#[test]
fn test_checked_powf_y_zero() {
    // x^0 = 1 for any x, via the y==0 fast path.
    let e = Env::default();
    let x = Wad::from_integer(&e, 42);
    assert_eq!(x.checked_powf(&e, Wad::from_raw(0)), Some(Wad::from_integer(&e, 1)));
}

#[test]
fn test_checked_powf_one_base() {
    // 1^y = 1 for any y, via the x==1 fast path.
    let e = Env::default();
    let one = Wad::from_integer(&e, 1);
    let weird_y = Wad::from_ratio(&e, 355, 113); // π-ish, definitely non-integer
    assert_eq!(one.checked_powf(&e, weird_y), Some(one));
}

#[test]
fn test_checked_powf_y_one() {
    // x^1 = x for any x, via the y==1 fast path.
    let e = Env::default();
    let x = Wad::from_ratio(&e, 7, 13);
    let one = Wad::from_integer(&e, 1);
    assert_eq!(x.checked_powf(&e, one), Some(x));
}

#[test]
fn test_checked_powf_integer_exponent_matches_powi() {
    // Integer-y fast path: checked_powf delegates to checked_powi when the
    // exponent is an exact non-negative integer in u32 range.
    let e = Env::default();
    let two = Wad::from_integer(&e, 2);
    let three_wad = Wad::from_integer(&e, 3);
    assert_eq!(two.checked_powf(&e, three_wad), two.checked_powi(&e, 3));
    assert_eq!(two.checked_powf(&e, three_wad), Some(Wad::from_integer(&e, 8)));
}
