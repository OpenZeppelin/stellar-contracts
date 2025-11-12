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

use soroban_sdk::{panic_with_error, Env, I256};

use crate::math::soroban_fixed_point::{SorobanFixedPoint, SorobanFixedPointError};

/// Performs floor(r / z)
pub fn div_floor(r: i128, z: i128) -> Option<i128> {
    if r < 0 || (r > 0 && z < 0) {
        // ceiling is taken by default for a negative result
        let remainder = r.checked_rem_euclid(z)?;
        (r / z).checked_sub(if remainder > 0 { 1 } else { 0 })
    } else {
        // floor taken by default for a positive or zero result
        r.checked_div(z)
    }
}

/// Performs ceil(r / z)
pub fn div_ceil(r: i128, z: i128) -> Option<i128> {
    if r <= 0 || z < 0 {
        // ceiling is taken by default for a negative or zero result
        r.checked_div(z)
    } else {
        // floor taken by default for a positive result
        let remainder = r.checked_rem_euclid(z)?;
        (r / z).checked_add(if remainder > 0 { 1 } else { 0 })
    }
}

impl SorobanFixedPoint for i128 {
    fn fixed_mul_floor(&self, env: &Env, y: &i128, denominator: &i128) -> i128 {
        scaled_mul_div_floor(self, env, y, denominator)
    }

    fn fixed_mul_ceil(&self, env: &Env, y: &i128, denominator: &i128) -> i128 {
        scaled_mul_div_ceil(self, env, y, denominator)
    }
}

/// Performs floor(x * y / z)
pub fn scaled_mul_div_floor(x: &i128, env: &Env, y: &i128, z: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_floor(r, *z)
            .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::ZeroDenominator)),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::mul_div_floor(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.to_i128()
                .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::ResultOverflow))
        }
    }
}

/// Performs floor(x * y / z)
pub fn scaled_mul_div_ceil(x: &i128, env: &Env, y: &i128, z: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_ceil(r, *z)
            .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::ZeroDenominator)),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::mul_div_ceil(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.to_i128()
                .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::ResultOverflow))
        }
    }
}
