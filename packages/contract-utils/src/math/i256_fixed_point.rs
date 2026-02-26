// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: I256 arithmetic operations in the soroban-sdk do not have checked
// variants as of yet, the `checked` variants in this codebase may result in
// panicking behavior.
//
// NOTE: Unlike the i128 variants, phantom overflow is NOT handled here.
// Handling I256 multiplication overflow would require a custom I512 type.
// Overflowing two large I256 values is considered rare enough in practice
// that this trade-off is acceptable.

use soroban_sdk::{Env, I256};

use crate::math::Rounding;

/// Calculates `x * y / denominator` following the specified rounding direction.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
pub fn mul_div_with_rounding(x: I256, y: I256, denominator: I256, rounding: Rounding) -> I256 {
    match rounding {
        Rounding::Floor => mul_div_floor(&x, &y, &denominator),
        Rounding::Ceil => mul_div_ceil(&x, &y, &denominator),
        Rounding::Truncate => mul_div(&x, &y, &denominator),
    }
}

/// Checked version of [`mul_div_with_rounding_i256`].
///
/// Calculates `x * y / denominator`, returning `None` on division by zero or
/// division overflow instead of panicking.
///
/// # Current Limitations
///
/// The intermediate `x * y` multiplication uses `I256::mul`, which panics on
/// overflow because `soroban-sdk` does not yet provide a checked multiply for
/// `I256`. Once a checked variant becomes available, this function will return
/// `None` for multiplication overflow as well.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
pub fn checked_mul_div_with_rounding(
    x: I256,
    y: I256,
    denominator: I256,
    rounding: Rounding,
) -> Option<I256> {
    match rounding {
        Rounding::Floor => checked_mul_div_floor(&x, &y, &denominator),
        Rounding::Ceil => checked_mul_div_ceil(&x, &y, &denominator),
        Rounding::Truncate => checked_mul_div(&x, &y, &denominator),
    }
}

/// Calculates floor(x * y / denominator).
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn mul_div_floor(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let r = x.mul(y);
    div_floor(&r, denominator)
}

/// Calculates ceil(x * y / denominator).
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn mul_div_ceil(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let r = x.mul(y);
    div_ceil(&r, denominator)
}

/// Calculates `x * y / denominator` (truncated toward zero).
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn mul_div(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let r = x.mul(y);
    r.div(denominator)
}

/// Calculates floor(x * y / denominator).
///
/// Returns `None` if `denominator` is zero or if the division overflows.
///
/// # Current Limitations
///
/// The intermediate `x * y` multiplication uses `I256::mul`, which panics on
/// overflow because `soroban-sdk` does not yet provide a checked multiply for
/// `I256`. Once a checked variant becomes available, this function will return
/// `None` for multiplication overflow as well.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_floor(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let r = x.mul(y);
    checked_div_floor(&r, denominator)
}

/// Calculates ceil(x * y / denominator).
///
/// Returns `None` if `denominator` is zero or if the division overflows.
///
/// # Current Limitations
///
/// The intermediate `x * y` multiplication uses `I256::mul`, which panics on
/// overflow because `soroban-sdk` does not yet provide a checked multiply for
/// `I256`. Once a checked variant becomes available, this function will return
/// `None` for multiplication overflow as well.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_ceil(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let r = x.mul(y);
    checked_div_ceil(&r, denominator)
}

/// Calculates `x * y / denominator` (truncated toward zero).
///
/// Returns `None` if `denominator` is zero.
///
/// # Current Limitations
///
/// The intermediate `x * y` multiplication uses `I256::mul`, which panics on
/// overflow because `soroban-sdk` does not yet provide a checked multiply for
/// `I256`. Once a checked variant becomes available, this function will return
/// `None` for multiplication overflow as well.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let zero = I256::from_i32(x.env(), 0);

    // TODO: remove this check when `checked_div` is available: https://github.com/stellar/rs-soroban-sdk/issues/1659,
    if *denominator == zero {
        return None;
    }

    let r = x.mul(y);
    Some(r.div(denominator))
}

// ###################### HELPERS ######################

// TODO: use the checked variants of `rem_euclid`, `div`, and `sub` below when they are available: https://github.com/stellar/rs-soroban-sdk/issues/1659,

/// Performs checked floor(r / z)
fn checked_div_floor(r: &I256, z: &I256) -> Option<I256> {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);

    if z == zero {
        return None;
    }

    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
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
    let env = r.env();
    let zero = &I256::from_i32(env, 0);
    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).sub(if remainder > *zero { &one } else { zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.div(z)
    }
}

/// Performs checked ceil(r / z)
fn checked_div_ceil(r: &I256, z: &I256) -> Option<I256> {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);

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
        let one = I256::from_i32(env, 1);
        Some(r.div(z).add(if remainder > *zero { &one } else { zero }))
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: &I256, z: &I256) -> I256 {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);
    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        r.div(z)
    } else {
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).add(if remainder > *zero { &one } else { zero })
    }
}

/// check I256 div overflow
fn check_div_overflow(r: &I256, z: &I256) -> bool {
    let env = r.env();
    let i256_min = i256_min(env);
    let neg_one = I256::from_i32(env, -1);
    r == &i256_min && z == &neg_one
}

/// Returns the minimum representable i256 value: -2^255.
///
/// The I256 is constructed from 4 parts (big-endian order): hi_hi: i64, hi_lo:
/// u64, lo_hi: u64, lo_lo: u64. The minimum i256 value (-2^255) in two's
/// complement is: Bit pattern: 1 followed by 255 zeros
/// That means: hi_hi = 0x8000000000000000 (which is i64::MIN), and all other
/// parts = 0
///
/// Replace with `I256::MIN` once https://github.com/stellar/stellar-protocol/issues/1885 is fixed
fn i256_min(e: &Env) -> I256 {
    I256::from_parts(e, i64::MIN, 0, 0, 0)
}
