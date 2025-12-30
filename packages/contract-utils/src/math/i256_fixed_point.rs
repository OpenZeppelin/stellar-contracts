// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: I256 arithmetic operations in the soroban-sdk do not have checked
// variants as of yet, the `checked` variants in this codebase may result in
// panicking behavior.

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
pub fn mul_div_i256(e: &Env, x: I256, y: I256, denominator: I256, rounding: Rounding) -> I256 {
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
pub fn checked_mul_div_i256(
    e: &Env,
    x: I256,
    y: I256,
    denominator: I256,
    rounding: Rounding,
) -> Option<I256> {
    match rounding {
        Rounding::Floor => x.checked_mul_div_floor(e, &y, &denominator),
        Rounding::Ceil => x.checked_mul_div_ceil(e, &y, &denominator),
        Rounding::Truncate => x.checked_mul_div(e, &y, &denominator),
    }
}

impl SorobanMulDiv for I256 {
    /// Calculates floor(x * y / denominator).
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
    fn mul_div_floor(&self, e: &Env, y: &I256, denominator: &I256) -> I256 {
        let zero = I256::from_i32(e, 0);

        if *denominator == zero {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }

        let r = self.mul(y);

        div_floor(&r, denominator)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Calculates ceil(x * y / denominator).
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
    fn mul_div_ceil(&self, e: &Env, y: &I256, denominator: &I256) -> I256 {
        let zero = I256::from_i32(e, 0);

        if *denominator == zero {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }

        let r = self.mul(y);

        div_ceil(&r, denominator)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Calculates (x * y / denominator).
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
    fn mul_div(&self, e: &Env, y: &I256, denominator: &I256) -> I256 {
        let zero = I256::from_i32(e, 0);

        if *denominator == zero {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }

        let r = self.mul(y);
        r.div(denominator)
    }

    /// Calculates floor(x * y / denominator).
    ///
    /// Returns `None` if denominator is zero or if the result overflows.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_floor(&self, e: &Env, y: &I256, denominator: &I256) -> Option<I256> {
        let zero = I256::from_i32(e, 0);

        // TODO: remove this check when `checked_div` is available: https://github.com/stellar/rs-soroban-sdk/issues/1659,
        if *denominator == zero {
            return None;
        }

        let r = self.mul(y);

        div_floor(&r, denominator)
    }

    /// Calculates ceil(x * y / denominator).
    ///
    /// Returns `None` if denominator is zero or if the result overflows.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_ceil(&self, e: &Env, y: &I256, denominator: &I256) -> Option<I256> {
        let zero = I256::from_i32(e, 0);

        // TODO: remove this check when `checked_div` is available: https://github.com/stellar/rs-soroban-sdk/issues/1659,
        if *denominator == zero {
            return None;
        }

        let r = self.mul(y);

        div_ceil(&r, denominator)
    }

    /// Calculates (x * y / denominator).
    ///
    /// Returns `None` if denominator is zero or if the result overflows.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div(&self, e: &Env, y: &I256, denominator: &I256) -> Option<I256> {
        let zero = I256::from_i32(e, 0);

        // TODO: remove this check when `checked_div` is available: https://github.com/stellar/rs-soroban-sdk/issues/1659,
        if *denominator == zero {
            return None;
        }

        let r = self.mul(y);
        Some(r.div(denominator))
    }
}

// ###################### HELPERS ######################

// TODO: use the checked variants below when they are available: https://github.com/stellar/rs-soroban-sdk/issues/1659,

/// Performs floor(r / z)
fn div_floor(r: &I256, z: &I256) -> Option<I256> {
    let zero = &I256::from_i32(&Env::default(), 0);
    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        Some(r.div(z).sub(if remainder > *zero { &one } else { zero }))
    } else {
        // floor is taken by default for a positive or zero result
        Some(r.div(z))
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: &I256, z: &I256) -> Option<I256> {
    let zero = &I256::from_i32(&Env::default(), 0);
    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        Some(r.div(z))
    } else {
        // floor is taken by default for a positive result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        Some(r.div(z).add(if remainder > *zero { &one } else { zero }))
    }
}
