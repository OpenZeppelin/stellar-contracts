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

impl SorobanFixedPoint for I256 {
    fn fixed_mul_floor(&self, env: &Env, y: &I256, denominator: &I256) -> I256 {
        mul_div_floor(env, self, y, denominator)
    }

    fn fixed_mul_ceil(&self, env: &Env, y: &I256, denominator: &I256) -> I256 {
        mul_div_ceil(env, self, y, denominator)
    }
}

/// Performs floor(x * y / z)
pub(crate) fn mul_div_floor(env: &Env, x: &I256, y: &I256, z: &I256) -> I256 {
    let zero = I256::from_i32(env, 0);
    let r = x.mul(y);

    if z.clone() == zero {
        panic_with_error!(env, SorobanFixedPointError::ZeroDenominator);
    }

    if r < zero || (r > zero && z.clone() < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).sub(if remainder > zero { &one } else { &zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.div(z)
    }
}

/// Performs ceil(x * y / z)
pub(crate) fn mul_div_ceil(env: &Env, x: &I256, y: &I256, z: &I256) -> I256 {
    let zero = I256::from_i32(env, 0);
    let r = x.mul(y);

    if z.clone() == zero {
        panic_with_error!(env, SorobanFixedPointError::ZeroDenominator);
    }

    if z.clone() < zero || r <= zero {
        // ceil is taken by default for a negative or zero result
        r.div(z)
    } else {
        // floor is taken by default for a positive result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).add(if remainder > zero { &one } else { &zero })
    }
}
