/*
MIT License

Copyright (c) 2023 Script3 Ltd. and contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

#![cfg(test)]

extern crate std;

#[cfg(test)]
mod test_soroban_fixed_point {

    use soroban_sdk::Env;

    use crate::math::soroban_fixed_point::SorobanFixedPoint;

    /********** fixed_mul_floor ********* */

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
}

#[cfg(test)]
mod test_muldiv {
    use soroban_sdk::Env;

    use crate::math::fixed_point::{muldiv, Rounding};

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
    #[should_panic(expected = "Error(Contract, #1500)")]
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
}

#[cfg(test)]
mod test_i128_errors {
    use soroban_sdk::Env;

    use crate::math::soroban_fixed_point::SorobanFixedPoint;

    #[test]
    #[should_panic(expected = "Error(Contract, #1500)")]
    fn test_fixed_mul_floor_zero_denominator() {
        let env = Env::default();
        let x: i128 = 100;
        let y: i128 = 50;
        let denominator: i128 = 0;

        x.fixed_mul_floor(&env, &y, &denominator);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1500)")]
    fn test_fixed_mul_ceil_zero_denominator() {
        let env = Env::default();
        let x: i128 = 100;
        let y: i128 = 50;
        let denominator: i128 = 0;

        x.fixed_mul_ceil(&env, &y, &denominator);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1502)")]
    fn test_fixed_mul_floor_result_overflow() {
        let env = Env::default();
        // This will overflow i128 even after scaling to I256
        let x: i128 = i128::MAX;
        let y: i128 = i128::MAX;
        let denominator: i128 = 1;

        x.fixed_mul_floor(&env, &y, &denominator);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1502)")]
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
}

#[cfg(test)]
mod test_i256 {

    /********** fixed_mul_floor ********* */

    use soroban_sdk::{Env, I256};

    use crate::math::soroban_fixed_point::SorobanFixedPoint;

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
}

#[cfg(test)]
mod test_i256_errors {
    use soroban_sdk::{Env, I256};

    use crate::math::soroban_fixed_point::SorobanFixedPoint;

    #[test]
    #[should_panic(expected = "Error(Contract, #1500)")]
    fn test_fixed_mul_floor_zero_denominator() {
        let env = Env::default();
        let x: I256 = I256::from_i128(&env, 100);
        let y: I256 = I256::from_i128(&env, 50);
        let denominator: I256 = I256::from_i128(&env, 0);

        x.fixed_mul_floor(&env, &y, &denominator);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1500)")]
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
}

#[cfg(test)]
mod test_wad {
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
    #[should_panic(expected = "Error(Contract, #1600)")]
    fn test_from_integer_overflow() {
        let e = Env::default();
        let _ = Wad::from_integer(&e, i128::MAX);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1600)")]
    fn test_from_ratio_overflow() {
        let e = Env::default();
        let _ = Wad::from_ratio(&e, i128::MAX, 1);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1601)")]
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
        let result = a.checked_div(b);
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
    #[should_panic(expected = "Error(Contract, #1600)")]
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
        let result = a.checked_mul(b);
        assert_eq!(result, Some(Wad::from_integer(&e, 6)));
    }

    #[test]
    fn test_checked_mul_overflow() {
        let a = Wad::from_raw(i128::MAX);
        let b = Wad::from_raw(2);
        let result = a.checked_mul(b);
        assert_eq!(result, None);
    }

    #[test]
    fn test_checked_div_success() {
        let e = Env::default();
        let a = Wad::from_integer(&e, 6);
        let b = Wad::from_integer(&e, 2);
        let result = a.checked_div(b);
        assert_eq!(result, Some(Wad::from_integer(&e, 3)));
    }

    #[test]
    fn test_checked_div_overflow() {
        let a = Wad::from_raw(i128::MAX);
        let b = Wad::from_raw(1);
        let result = a.checked_div(b); // MAX * WAD_SCALE will overflow
        assert_eq!(result, None);
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
}
