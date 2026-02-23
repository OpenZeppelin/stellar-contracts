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
pub use i128_fixed_point::{checked_mul_div_with_rounding_i128, mul_div_with_rounding_i128};
pub use i256_fixed_point::{checked_mul_div_with_rounding_i256, mul_div_with_rounding_i256};

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contracttype};

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
    /// Round toward zero (truncation)
    Truncate,
}
