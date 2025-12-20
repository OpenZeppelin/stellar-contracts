// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

use soroban_sdk::{panic_with_error, Env, I256};

use crate::math::{Rounding, SorobanFixedPoint, SorobanFixedPointError};

/// Calculates `x * y / denominator` with full precision.
///
/// Performs multiplication and division with phantom overflow handling,
/// following the specified rounding direction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
///
/// # Errors
///
/// * [`SorobanFixedPointError::DivisionByZero`] - When `denominator` is zero.
/// * [`SorobanFixedPointError::Overflow`] - When the result overflows `i128`.
///
/// # Notes
///
/// Automatically handles phantom overflow by scaling to `I256` when necessary.
pub fn muldiv(e: &Env, x: i128, y: i128, denominator: i128, rounding: Rounding) -> i128 {
    match rounding {
        Rounding::Floor => x.fixed_mul_floor(e, &y, &denominator),
        Rounding::Ceil => x.fixed_mul_ceil(e, &y, &denominator),
    }
}

/// Checked version of [`muldiv`].
///
/// Calculates `x * y / denominator` with full precision, returning `None`
/// instead of panicking on error.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
///
/// # Notes
///
/// Automatically handles phantom overflow by scaling to `I256` when necessary.
pub fn checked_muldiv(
    e: &Env,
    x: i128,
    y: i128,
    denominator: i128,
    rounding: Rounding,
) -> Option<i128> {
    match rounding {
        Rounding::Floor => x.checked_fixed_mul_floor(e, &y, &denominator),
        Rounding::Ceil => x.checked_fixed_mul_ceil(e, &y, &denominator),
    }
}

/// Performs floor(r / z)
fn div_floor(r: i128, z: i128) -> Option<i128> {
    if (r < 0 && z > 0) || (r > 0 && z < 0) {
        // ceiling is taken by default for a negative result
        let remainder = r.checked_rem_euclid(z)?;
        (r / z).checked_sub(if remainder > 0 { 1 } else { 0 })
    } else {
        // floor taken by default for a positive or zero result
        r.checked_div(z)
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: i128, z: i128) -> Option<i128> {
    if (r <= 0 && z > 0) || (r >= 0 && z < 0) {
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

    fn checked_fixed_mul_floor(&self, env: &Env, y: &i128, denominator: &i128) -> Option<i128> {
        checked_scaled_mul_div_floor(self, env, y, denominator)
    }

    fn checked_fixed_mul_ceil(&self, env: &Env, y: &i128, denominator: &i128) -> Option<i128> {
        checked_scaled_mul_div_ceil(self, env, y, denominator)
    }
}

/// Performs floor(x * y / z), panics on overflow or division by zero
fn scaled_mul_div_floor(x: &i128, env: &Env, y: &i128, z: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_floor(r, *z)
            .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::DivisionByZero)),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::mul_div_floor(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.to_i128()
                .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::Overflow))
        }
    }
}

/// Performs ceil(x * y / z)
fn scaled_mul_div_ceil(x: &i128, env: &Env, y: &i128, z: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_ceil(r, *z)
            .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::DivisionByZero)),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::mul_div_ceil(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.to_i128()
                .unwrap_or_else(|| panic_with_error!(env, SorobanFixedPointError::Overflow))
        }
    }
}

/// Checked version of floor(x * y / z)
fn checked_scaled_mul_div_floor(x: &i128, env: &Env, y: &i128, z: &i128) -> Option<i128> {
    match x.checked_mul(*y) {
        Some(r) => div_floor(r, *z),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::checked_mul_div_floor(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.map(|r| r.to_i128())?
        }
    }
}

/// Checked version of ceil(x * y / z)
fn checked_scaled_mul_div_ceil(x: &i128, env: &Env, y: &i128, z: &i128) -> Option<i128> {
    match x.checked_mul(*y) {
        Some(r) => div_ceil(r, *z),
        None => {
            // scale to i256 and retry
            let res = crate::math::i256_fixed_point::checked_mul_div_ceil(
                env,
                &I256::from_i128(env, *x),
                &I256::from_i128(env, *y),
                &I256::from_i128(env, *z),
            );
            res.map(|r| r.to_i128())?
        }
    }
}
