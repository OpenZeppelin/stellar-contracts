// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: I256 arithmetic operations in the soroban-sdk do not have checked
// variants as of yet, the `checked` variants in this codebase may result in
// panicking behavior.

use soroban_sdk::{Env, I256};

use crate::math::{Rounding, SorobanMulDiv};

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
/// * refer to the errors of [`I256::mul_div_floor`]
/// * refer to the errors of [`I256::mul_div_ceil`]
/// * refer to the errors of [`I256::mul_div`]
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
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn mul_div_floor(&self, y: &I256, denominator: &I256) -> I256 {
        let r = self.mul(y);
        div_floor(&r, denominator)
    }

    /// Calculates ceil(x * y / denominator).
    ///
    /// # Arguments
    ///
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn mul_div_ceil(&self, y: &I256, denominator: &I256) -> I256 {
        let r = self.mul(y);
        div_ceil(&r, denominator)
    }

    /// Calculates (x * y / denominator).
    ///
    /// # Arguments
    ///
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn mul_div(&self, y: &I256, denominator: &I256) -> I256 {
        let r = self.mul(y);
        r.div(denominator)
    }

    /// Calculates floor(x * y / denominator).
    ///
    /// Returns `None` if denominator is zero or if the result overflows.
    ///
    /// # Arguments
    ///
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_floor(&self, y: &I256, denominator: &I256) -> Option<I256> {
        let r = self.mul(y);
        checked_div_floor(&r, denominator)
    }

    /// Calculates ceil(x * y / denominator).
    ///
    /// Returns `None` if denominator is zero or if the result overflows.
    ///
    /// # Arguments
    ///
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_mul_div_ceil(&self, y: &I256, denominator: &I256) -> Option<I256> {
        let r = self.mul(y);
        checked_div_ceil(&r, denominator)
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

// TODO: use the checked variants of `rem_euclid`, `div`, and `sub` below when they are available: https://github.com/stellar/rs-soroban-sdk/issues/1659,

/// Performs checked floor(r / z)
fn checked_div_floor(r: &I256, z: &I256) -> Option<I256> {
    let zero = &I256::from_i32(&Env::default(), 0);

    if z == zero {
        return None;
    }

    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        Some(r.div(z).sub(if remainder > *zero { &one } else { zero }))
    } else {
        // floor is taken by default for a positive or zero result
        if check_div_overflow(r, z) {
            return None;
        }

        Some(r.div(z))
    }
}

/// Performs floor(r / z)
fn div_floor(r: &I256, z: &I256) -> I256 {
    let zero = &I256::from_i32(&Env::default(), 0);
    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        r.div(z).sub(if remainder > *zero { &one } else { zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.div(z)
    }
}

/// Performs checked ceil(r / z)
fn checked_div_ceil(r: &I256, z: &I256) -> Option<I256> {
    let zero = &I256::from_i32(&Env::default(), 0);

    if z == zero {
        return None;
    }

    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        Some(r.div(z))
    } else {
        // floor is taken by default for a positive result
        if check_div_overflow(r, z) {
            return None;
        }

        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        Some(r.div(z).add(if remainder > *zero { &one } else { zero }))
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: &I256, z: &I256) -> I256 {
    let zero = &I256::from_i32(&Env::default(), 0);
    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        r.div(z)
    } else {
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(&Env::default(), 1);
        r.div(z).add(if remainder > *zero { &one } else { zero })
    }
}

/// check I256 div overflow
fn check_div_overflow(r: &I256, z: &I256) -> bool {
    let i256_min = i256_min();
    let neg_one = I256::from_i32(&Env::default(), -1);
    r == &i256_min && z == &neg_one
}

/// Returns the minimum representable i256 value: -2^255.
///
/// The I256 is constructed from 4 parts (big-endian order): hi_hi: i64, hi_lo: u64, lo_hi: u64, lo_lo: u64.
/// The minimum i256 value (-2^255) in two's complement is:
/// Bit pattern: 1 followed by 255 zeros
/// That means: hi_hi = 0x8000000000000000 (which is i64::MIN), and all other parts = 0
///
/// Replace with `I256::MIN` once https://github.com/stellar/stellar-protocol/issues/1885 is fixed
fn i256_min() -> I256 {
    I256::from_parts(&Env::default(), i64::MIN, 0, 0, 0)
}
