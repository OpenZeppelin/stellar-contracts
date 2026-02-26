//! # Fixed-Point Math Library
//!
//! Provides utilities for precise fixed-point arithmetic operations in Soroban
//! smart contracts.
//!
//! ## Design Overview
//!
//! The library exposes free functions for `i128` and `I256` fixed-point
//! multiplication and division, in both panicking and checked variants:
//!
//! - **Panicking variants** (e.g. [`mul_div_with_rounding_i128`]): panic on
//!   overflow or division by zero with a [`SorobanFixedPointError`].
//! - **Checked variants** (e.g. [`checked_mul_div_with_rounding_i128`]): return
//!   `None` on error for graceful handling. Note that for `I256` operations,
//!   the intermediate `x * y` multiplication still panics on overflow because
//!   `soroban-sdk` does not yet provide a checked multiply for `I256`; once it
//!   does, `I256` checked variants will also return `None` for that case.
//!
//! ### Phantom Overflow Handling
//!
//! For `i128` operations, intermediate multiplication overflow is handled
//! transparently: when `x * y` overflows `i128`, the calculation is retried
//! using `I256` as an intermediate type and scaled back to `i128` if the final
//! result fits. This is called *phantom overflow handling*.
//!
//! `I256` operations do **not** apply phantom overflow handling — doing so
//! would require a custom `I512` type. Overflowing two large `I256` values is
//! considered rare enough in practice that this trade-off is acceptable.
//!
//! ## Structure
//!
//! - [`i128_fixed_point`]: Module containing free functions for `i128`
//!   fixed-point multiplication and division.
//! - [`i256_fixed_point`]: Module containing free functions for `I256`
//!   fixed-point multiplication and division.
//! - [`wad`]: Fixed-point decimal number type with 18 decimal places.
//! - [`Rounding`]: Enum to specify rounding direction (floor, ceil, truncate).
//! - [`SorobanFixedPointError`]: Error codes emitted by panicking variants.
//!
//! ## Notes
//!
//! Based on the Soroban fixed-point mathematics library.
//! Original implementation: <https://github.com/script3/soroban-fixed-point-math>

pub mod i128_fixed_point;
pub mod i256_fixed_point;
pub mod wad;

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
