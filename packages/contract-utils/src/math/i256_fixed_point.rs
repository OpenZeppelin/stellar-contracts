// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: Unlike the i128 variants, phantom overflow is NOT resolved here.
// The i128 helpers promote to I256 so that an intermediate `x * y` which
// overflows i128 can still produce a correct result; doing the same for I256
// would require a custom I512 type. As a consequence, the `checked_*` functions
// return `None` when `x * y` overflows I256 (even if `x * y / denominator`
// would fit), and the unchecked functions panic in that case. Overflowing two
// large I256 values is considered rare enough in practice that this trade-off
// is acceptable.

use soroban_sdk::I256;

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

/// Checked version of [`mul_div_with_rounding`].
///
/// Calculates `x * y / denominator`, returning `None` instead of panicking
/// when the intermediate `x * y` overflows `I256`, when `denominator` is zero,
/// or when the division overflows.
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
/// Returns `None` if the intermediate `x * y` overflows `I256`, if
/// `denominator` is zero, or if the division overflows.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_floor(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let r = x.checked_mul(y)?;
    checked_div_floor(&r, denominator)
}

/// Calculates ceil(x * y / denominator).
///
/// Returns `None` if the intermediate `x * y` overflows `I256`, if
/// `denominator` is zero, or if the division overflows.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_ceil(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let r = x.checked_mul(y)?;
    checked_div_ceil(&r, denominator)
}

/// Calculates `x * y / denominator` (truncated toward zero).
///
/// Returns `None` if the intermediate `x * y` overflows `I256`, if
/// `denominator` is zero, or if the division overflows.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    let r = x.checked_mul(y)?;
    r.checked_div(denominator)
}

// ###################### HELPERS ######################

/// Performs checked floor(r / z)
fn checked_div_floor(r: &I256, z: &I256) -> Option<I256> {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);

    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.checked_rem_euclid(z)?;
        let one = I256::from_i32(env, 1);
        let quotient = r.checked_div(z)?;
        quotient.checked_sub(if remainder > *zero { &one } else { zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.checked_div(z)
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

    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        r.checked_div(z)
    } else {
        // floor is taken by default for a positive result
        let remainder = r.checked_rem_euclid(z)?;
        let one = I256::from_i32(env, 1);
        let quotient = r.checked_div(z)?;
        quotient.checked_add(if remainder > *zero { &one } else { zero })
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
