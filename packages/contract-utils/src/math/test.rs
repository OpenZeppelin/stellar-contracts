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
