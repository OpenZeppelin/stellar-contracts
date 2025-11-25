//! # Fixed-Point Math Library
//!
//! Provides utilities for precise fixed-point arithmetic operations in Soroban
//! smart contracts.
//!
//! This module implements fixed-point multiplication and division with phantom
//! overflow handling, ensuring accurate calculations even when intermediate
//! results temporarily exceed the native integer type bounds.
//!
//! ## Design Overview
//!
//! The library is built around the [`SorobanFixedPoint`] trait, which provides
//! both panicking and checked variants of fixed-point operations:
//!
//! - **Panicking Variants** (`fixed_mul_floor`, `fixed_mul_ceil`): Panic on
//!   errors with specific error types ([`SorobanFixedPointError`]).
//! - **Checked Variants** (`checked_fixed_mul_floor`,
//!   `checked_fixed_mul_ceil`): Return `None` on errors for graceful error
//!   handling.
//!
//! ### Phantom Overflow Handling
//!
//! A key feature is automatic phantom overflow handling. When multiplying two
//! `i128` values would overflow, the implementation automatically scales up to
//! `I256` for the intermediate calculation, then scales back down if the final
//! result fits in `i128`.
//!
//! ## Structure
//!
//! The module includes:
//!
//! - [`SorobanFixedPoint`]: Core trait implemented for `i128` and `I256`.
//! - [`wad`]: Fixed-point decimal number type with 18 decimal places.
//! - [`muldiv`] and [`checked_muldiv`]: Public API functions for common
//!   operations.
//! - [`Rounding`]: Enum to specify rounding direction (floor or ceil).
//!
//! ## Notes
//!
//! Based on the Soroban fixed-point mathematics library.
//! Original implementation: <https://github.com/script3/soroban-fixed-point-math>

mod i128_fixed_point;
mod i256_fixed_point;
pub mod wad;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contracttype, Env};

/// Trait for computing fixed-point calculations with Soroban host objects.
///
/// Provides both panicking and checked variants for floor and ceiling division
/// operations. Implementations automatically handle phantom overflow by scaling
/// to larger integer types when necessary.
pub trait SorobanFixedPoint: Sized {
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
    /// * [`SorobanFixedPointError::DivisionByZero`] - When `denominator` is
    ///   zero.
    /// * [`SorobanFixedPointError::Overflow`] - When a phantom overflow occurs
    ///   or the result does not fit in `Self`.
    fn fixed_mul_floor(&self, env: &Env, y: &Self, denominator: &Self) -> Self;

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
    /// * [`SorobanFixedPointError::DivisionByZero`] - When `denominator` is
    ///   zero.
    /// * [`SorobanFixedPointError::Overflow`] - When a phantom overflow occurs
    ///   or the result does not fit in `Self`.
    fn fixed_mul_ceil(&self, env: &Env, y: &Self, denominator: &Self) -> Self;

    /// Checked version of floor(x * y / denominator).
    ///
    /// Returns `None` instead of panicking on error.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
    fn checked_fixed_mul_floor(&self, env: &Env, y: &Self, denominator: &Self) -> Option<Self>;

    /// Checked version of ceil(x * y / denominator).
    ///
    /// Returns `None` instead of panicking on error.
    ///
    /// # Arguments
    ///
    /// * `env` - Access to the Soroban environment.
    /// * `y` - The multiplicand.
    /// * `denominator` - The divisor.
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

/// Rounding direction for division operations
#[contracttype]
pub enum Rounding {
    /// Round toward negative infinity (down)
    Floor,
    /// Round toward positive infinity (up)
    Ceil,
}

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
