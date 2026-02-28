// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

use soroban_sdk::{panic_with_error, Env, I256};

use crate::math::{
    i256_fixed_point::{
        checked_mul_div as checked_mul_div_i256, checked_mul_div_ceil as checked_mul_div_ceil_i256,
        checked_mul_div_floor as checked_mul_div_floor_i256, mul_div as mul_div_i256,
        mul_div_ceil as mul_div_ceil_i256, mul_div_floor as mul_div_floor_i256,
    },
    Rounding, SorobanFixedPointError,
};

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
/// * refer to the errors of [`mul_div_floor_i128`]
/// * refer to the errors of [`mul_div_ceil_i128`]
/// * refer to the errors of [`mul_div_i128`]
///
/// # Notes
///
/// Automatically handles phantom overflow by scaling to `I256` when necessary.
pub fn mul_div_with_rounding(
    e: &Env,
    x: i128,
    y: i128,
    denominator: i128,
    rounding: Rounding,
) -> i128 {
    match rounding {
        Rounding::Floor => mul_div_floor(e, &x, &y, &denominator),
        Rounding::Ceil => mul_div_ceil(e, &x, &y, &denominator),
        Rounding::Truncate => mul_div(e, &x, &y, &denominator),
    }
}

/// Checked version of [`mul_div_with_rounding_i128`].
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
pub fn checked_mul_div_with_rounding(
    e: &Env,
    x: i128,
    y: i128,
    denominator: i128,
    rounding: Rounding,
) -> Option<i128> {
    match rounding {
        Rounding::Floor => checked_mul_div_floor(e, &x, &y, &denominator),
        Rounding::Ceil => checked_mul_div_ceil(e, &x, &y, &denominator),
        Rounding::Truncate => checked_mul_div(e, &x, &y, &denominator),
    }
}

/// Calculates floor(x * y / denominator) with automatic scaling to I256
/// when necessary.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
/// * if `denominator` is zero, it will panic due to standard library behavior.
pub fn mul_div_floor(e: &Env, x: &i128, y: &i128, denominator: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_floor(r, *denominator),
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = mul_div_floor_i256(x_i256, y_i256, z_i256);

            res.to_i128().unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
        }
    }
}

/// Calculates ceil(x * y / denominator) with automatic scaling to I256 when
/// necessary.
///
/// # Arguments
///
/// * `env` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
/// * if `denominator` is zero, it will panic due to standard library behavior.
pub fn mul_div_ceil(e: &Env, x: &i128, y: &i128, denominator: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => div_ceil(r, *denominator),
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = mul_div_ceil_i256(x_i256, y_i256, z_i256);

            res.to_i128().unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
        }
    }
}

/// Calculates (x * y / denominator) with automatic scaling to I256 when
/// necessary.
///
/// # Arguments
///
/// * `env` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
/// * if `denominator` is zero, it will panic due to standard library behavior.
pub fn mul_div(e: &Env, x: &i128, y: &i128, denominator: &i128) -> i128 {
    match x.checked_mul(*y) {
        Some(r) => r / *denominator,
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = mul_div_i256(x_i256, y_i256, z_i256);

            res.to_i128().unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
        }
    }
}

/// Checked version of floor(x * y / denominator).
///
/// Returns `None` if the result overflows or if `denominator` is zero.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_floor(e: &Env, x: &i128, y: &i128, denominator: &i128) -> Option<i128> {
    match x.checked_mul(*y) {
        Some(r) => checked_div_floor(r, *denominator),
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = checked_mul_div_floor_i256(x_i256, y_i256, z_i256);

            res.map(|r| r.to_i128())?
        }
    }
}

/// Checked version of ceil(x * y / denominator).
///
/// Returns `None` if the result overflows or if `denominator` is zero.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_ceil(e: &Env, x: &i128, y: &i128, denominator: &i128) -> Option<i128> {
    match x.checked_mul(*y) {
        Some(r) => checked_div_ceil(r, *denominator),
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = checked_mul_div_ceil_i256(x_i256, y_i256, z_i256);

            res.map(|r| r.to_i128())?
        }
    }
}

/// Checked version of (x * y / denominator).
///
/// Returns `None` if the result overflows or if `denominator` is zero.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div(e: &Env, x: &i128, y: &i128, denominator: &i128) -> Option<i128> {
    match x.checked_mul(*y) {
        Some(r) => r.checked_div(*denominator),
        None => {
            // scale to i256 and retry
            let x_i256 = &I256::from_i128(e, *x);
            let y_i256 = &I256::from_i128(e, *y);
            let z_i256 = &I256::from_i128(e, *denominator);

            let res = checked_mul_div_i256(x_i256, y_i256, z_i256);

            res.map(|r| r.to_i128())?
        }
    }
}

// ###################### HELPERS ######################

/// Performs checked floor(r / z)
fn checked_div_floor(r: i128, z: i128) -> Option<i128> {
    if (r < 0 && z > 0) || (r > 0 && z < 0) {
        // ceiling is taken by default for a negative result
        let remainder = r.checked_rem_euclid(z)?;

        // no need to check for div overflow (i128::MIN / -1),
        // because it doesn't fall under this if branch
        (r / z).checked_sub(if remainder > 0 { 1 } else { 0 })
    } else {
        // floor taken by default for a positive or zero result
        r.checked_div(z)
    }
}

/// Performs floor(r / z)
fn div_floor(r: i128, z: i128) -> i128 {
    if (r < 0 && z > 0) || (r > 0 && z < 0) {
        // ceiling is taken by default for a negative result
        let remainder = r.rem_euclid(z);

        (r / z) - (if remainder > 0 { 1 } else { 0 })
    } else {
        // floor taken by default for a positive or zero result
        r / z
    }
}

/// Performs checked ceil(r / z)
fn checked_div_ceil(r: i128, z: i128) -> Option<i128> {
    if (r <= 0 && z > 0) || (r >= 0 && z < 0) {
        // ceiling is taken by default for a negative or zero result
        r.checked_div(z)
    } else {
        // floor taken by default for a positive result
        let remainder = r.checked_rem_euclid(z)?;

        // check for div overflow (i128::MIN / -1)
        r.checked_div(z)?.checked_add(if remainder > 0 { 1 } else { 0 })
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: i128, z: i128) -> i128 {
    if (r <= 0 && z > 0) || (r >= 0 && z < 0) {
        // ceiling is taken by default for a negative or zero result
        r / z
    } else {
        // floor taken by default for a positive result
        let remainder = r.rem_euclid(z);

        r / z + (if remainder > 0 { 1 } else { 0 })
    }
}
