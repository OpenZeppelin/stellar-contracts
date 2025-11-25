mod i128_fixed_point;
mod i256_fixed_point;
pub mod wad;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contracttype, Env};
// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// @dev - more detail about the forced panic can be found here: https://github.com/stellar/rs-soroban-env/pull/1091
//
/// Soroban fixed point trait for computing fixed point calculations with
/// Soroban host objects.
///
/// Soroban host functions by default are non-recoverable. This means an
/// arithmetic overflow or divide by zero will result in a host panic instead of
/// returning an error. For consistency, this trait will also panic in the same
/// manner.
pub trait SorobanFixedPoint: Sized {
    /// Safely calculates floor(x * y / denominator).
    ///
    /// ### Panics
    /// This method will panic if the denominator is 0, a phantom overflow
    /// occurs, or the result does not fit in Self.
    fn fixed_mul_floor(&self, env: &Env, y: &Self, denominator: &Self) -> Self;

    /// Safely calculates ceil(x * y / denominator).
    ///
    /// ### Panics
    /// This method will panic if the denominator is 0, a phantom overflow
    /// occurs, or the result does not fit in Self.
    fn fixed_mul_ceil(&self, env: &Env, y: &Self, denominator: &Self) -> Self;

    /// Checked version of floor(x * y / denominator).
    ///
    /// Returns `None` if the denominator is 0, a phantom overflow occurs,
    /// or the result does not fit in Self.
    fn checked_fixed_mul_floor(&self, env: &Env, y: &Self, denominator: &Self) -> Option<Self>;

    /// Checked version of ceil(x * y / denominator).
    ///
    /// Returns `None` if the denominator is 0, a phantom overflow occurs,
    /// or the result does not fit in Self.
    fn checked_fixed_mul_ceil(&self, env: &Env, y: &Self, denominator: &Self) -> Option<Self>;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SorobanFixedPointError {
    /// Arithmetic overflow occurred
    Overflow = 1500,
    /// Division by zero
    DivisionByZero = 1501,
}

#[contracttype]
pub enum Rounding {
    Floor, // Toward negative infinity
    Ceil,  // Toward positive infinity
}

/**
 * Calculates x * y / denominator with full precision, following the
 * selected rounding direction. Throws if result overflows a i128 or
 * denominator is zero (handles phantom overflow).
 */
pub fn muldiv(e: &Env, x: i128, y: i128, denominator: i128, rounding: Rounding) -> i128 {
    match rounding {
        Rounding::Floor => x.fixed_mul_floor(e, &y, &denominator),
        Rounding::Ceil => x.fixed_mul_ceil(e, &y, &denominator),
    }
}

/**
 * Checked version of muldiv. Calculates x * y / denominator with full
 * precision, following the selected rounding direction. Returns None if
 * result overflows a i128 or denominator is zero (handles phantom
 * overflow).
 */
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
