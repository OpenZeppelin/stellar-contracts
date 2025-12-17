// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

use soroban_sdk::{panic_with_error, Env, I256};

use crate::math::{SorobanFixedPoint, SorobanFixedPointError};

impl SorobanFixedPoint for I256 {
    fn fixed_mul_floor(&self, env: &Env, y: &I256, denominator: &I256) -> I256 {
        mul_div_floor(env, self, y, denominator)
    }

    fn fixed_mul_ceil(&self, env: &Env, y: &I256, denominator: &I256) -> I256 {
        mul_div_ceil(env, self, y, denominator)
    }

    fn checked_fixed_mul_floor(&self, env: &Env, y: &I256, denominator: &I256) -> Option<I256> {
        checked_mul_div_floor(env, self, y, denominator)
    }

    fn checked_fixed_mul_ceil(&self, env: &Env, y: &I256, denominator: &I256) -> Option<I256> {
        checked_mul_div_ceil(env, self, y, denominator)
    }
}

/// Performs floor(x * y / z)
pub(crate) fn mul_div_floor(env: &Env, x: &I256, y: &I256, z: &I256) -> I256 {
    checked_mul_div_floor(env, x, y, z)
        .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::DivisionByZero))
}

/// Performs ceil(x * y / z)
pub(crate) fn mul_div_ceil(env: &Env, x: &I256, y: &I256, z: &I256) -> I256 {
    checked_mul_div_ceil(env, x, y, z)
        .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::DivisionByZero))
}

/// Checked version of floor(x * y / z)
pub(crate) fn checked_mul_div_floor(env: &Env, x: &I256, y: &I256, z: &I256) -> Option<I256> {
    let zero = I256::from_i32(env, 0);
    let r = x.mul(y);

    if *z == zero {
        return None;
    }

    if (r < zero && *z > zero) || (r > zero && *z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        Some(r.div(z).sub(if remainder > zero { &one } else { &zero }))
    } else {
        // floor is taken by default for a positive or zero result
        Some(r.div(z))
    }
}

/// Checked version of ceil(x * y / z)
pub(crate) fn checked_mul_div_ceil(env: &Env, x: &I256, y: &I256, z: &I256) -> Option<I256> {
    let zero = I256::from_i32(env, 0);
    let r = x.mul(y);

    if *z == zero {
        return None;
    }

    if (r <= zero && *z > zero) || (r >= zero && *z < zero) {
        // ceil is taken by default for a negative or zero result
        Some(r.div(z))
    } else {
        // floor is taken by default for a positive result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        Some(r.div(z).add(if remainder > zero { &one } else { &zero }))
    }
}
