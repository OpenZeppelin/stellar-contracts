// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

use soroban_sdk::{panic_with_error, Env, I256};

use crate::math::{Rounding, SorobanFixedPointError, SorobanMulDiv};

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
pub fn mul_div_i128(e: &Env, x: i128, y: i128, denominator: i128, rounding: Rounding) -> i128 {
    match rounding {
        Rounding::Floor => x.mul_div_floor(e, &y, &denominator),
        Rounding::Ceil => x.mul_div_ceil(e, &y, &denominator),
        Rounding::Truncate => x.mul_div(e, &y, &denominator),
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
pub fn checked_mul_div_i128(
    e: &Env,
    x: i128,
    y: i128,
    denominator: i128,
    rounding: Rounding,
) -> Option<i128> {
    match rounding {
        Rounding::Floor => x.checked_mul_div_floor(e, &y, &denominator),
        Rounding::Ceil => x.checked_mul_div_ceil(e, &y, &denominator),
        Rounding::Truncate => x.checked_mul_div(e, &y, &denominator),
    }
}

impl SorobanMulDiv for i128 {
    /// Calculates floor(x * y / denominator) with automatic scaling to I256
    /// when necessary.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::DivisionByZero`] - when `denominator` is
    ///   zero.
    /// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
    fn mul_div_floor(&self, e: &Env, y: &i128, denominator: &i128) -> i128 {
        if *denominator == 0 {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }
        match self.checked_mul(*y) {
            // *z == 0 check is already done above, so the only possible error is overflow,
            // where r = i128::MIN and z = -1
            Some(r) => div_floor(r, *denominator)
                .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)),
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.mul_div_floor(e, y_i256, z_i256);

                res.to_i128()
                    .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
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
    /// * [`SorobanFixedPointError::DivisionByZero`] - when `denominator` is
    ///   zero.
    /// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
    fn mul_div_ceil(&self, e: &Env, y: &i128, denominator: &i128) -> i128 {
        if *denominator == 0 {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }
        match self.checked_mul(*y) {
            // *z == 0 check is already done above, so the only possible error is overflow,
            // where r = i128::MIN and z = -1
            Some(r) => div_ceil(r, *denominator)
                .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)),
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.mul_div_ceil(e, y_i256, z_i256);

                res.to_i128()
                    .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
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
    /// * [`SorobanFixedPointError::DivisionByZero`] - when `denominator` is
    ///   zero.
    /// * [`SorobanFixedPointError::Overflow`] - when the result overflows.
    fn mul_div(&self, e: &Env, y: &i128, denominator: &i128) -> i128 {
        if *denominator == 0 {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }
        match self.checked_mul(*y) {
            Some(r) => r / *denominator,
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.mul_div(e, y_i256, z_i256);

                res.to_i128()
                    .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
            }
        }
    }

    /// Checked version of floor(x * y / denominator).
    ///
    /// Returns `None` if the result overflows or if `denominator` is zero.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_floor(&self, e: &Env, y: &i128, denominator: &i128) -> Option<i128> {
        match self.checked_mul(*y) {
            Some(r) => div_floor(r, *denominator),
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.checked_mul_div_floor(e, y_i256, z_i256);

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
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_ceil(&self, e: &Env, y: &i128, denominator: &i128) -> Option<i128> {
        match self.checked_mul(*y) {
            Some(r) => div_ceil(r, *denominator),
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.checked_mul_div_ceil(e, y_i256, z_i256);

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
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div(&self, e: &Env, y: &i128, denominator: &i128) -> Option<i128> {
        match self.checked_mul(*y) {
            Some(r) => r.checked_div(*denominator),
            None => {
                // scale to i256 and retry
                let x_i256 = &I256::from_i128(e, *self);
                let y_i256 = &I256::from_i128(e, *y);
                let z_i256 = &I256::from_i128(e, *denominator);

                let res = x_i256.checked_mul_div(e, y_i256, z_i256);

                res.map(|r| r.to_i128())?
            }
        }
    }
}

// ###################### HELPERS ######################

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
