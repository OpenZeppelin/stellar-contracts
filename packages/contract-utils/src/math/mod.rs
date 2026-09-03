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
//! - **Panicking variants** (e.g. [`i128_fixed_point::mul_div_with_rounding`]):
//!   panic with [`SorobanFixedPointError::Overflow`] when the result overflows.
//! - **Checked variants** (e.g.
//!   [`i128_fixed_point::checked_mul_div_with_rounding`]): return `None` on
//!   error for graceful handling, including when the intermediate `x * y`
//!   multiplication overflows and the result cannot be recovered.
//!
//! ### Phantom Overflow Handling
//!
//! For `i128` operations, intermediate multiplication overflow is handled
//! transparently: when `x * y` overflows `i128`, the calculation is retried
//! using `I256` as an intermediate type and scaled back to `i128` if the final
//! result fits. This is called *phantom overflow handling*.
//!
//! `I256` operations apply it too, without a wider intermediate type. When
//! `x * y` overflows `I256`, both operands are split by the denominator and the
//! division is distributed, which is an exact identity:
//!
//! ```text
//! |x| = q1*D + r1     |y| = q2*D + r2     (D = |denominator|, 0 <= r1, r2 < D)
//! floor(|x*y|/D) = q1*q2*D + q1*r2 + r1*q2 + floor(r1*r2/D)
//! ```
//!
//! The decomposition runs on magnitudes; the result's sign is reapplied before
//! the rounding direction is.
//!
//! Three of the four terms are bounded by the answer or by an input, so they
//! fit whenever the inputs and the result do. Only `r1*r2` is bounded by `D`
//! alone, which gives the single condition `|denominator| <= 2^128` (roughly
//! `3.4e38`, far above every fixed-point scale in practical use: `10^18`,
//! `10^27`, `2^96`, `10^38`). Within that domain the result is bit-for-bit what
//! a 512-bit intermediate would produce.
//!
//! Beyond it the operation rejects rather than returning an incorrect value,
//! and the rejection is permitted rather than guaranteed: a large denominator
//! whose remainders happen to be small still succeeds.
//!
//! Phantom overflow handling covers the free functions in
//! [`i128_fixed_point`] and [`i256_fixed_point`], and the `checked_*` methods
//! on [`wad::Wad`]. It does **not** cover `Wad`'s operator implementations
//! (`+`, `-`, `*`, `/`), which work directly on `i128` and panic on overflow,
//! because an operator cannot reach an `Env` to construct the `I256`
//! intermediate. The resulting bounds are tabulated in the `# Overflow`
//! section on [`wad::Wad`].
//!
//! ### Error Reporting
//!
//! Domain errors (a zero `denominator`, `MIN / -1`) are left to the platform
//! rather than mapped to contract errors, since these are plain arithmetic
//! operations. What the platform raises depends on the width:
//!
//! - **`i128`**: native Rust arithmetic, whose panics surface on-chain as a
//!   generic wasm trap (`Error(WasmVm, InvalidAction)`).
//! - **`I256`**: host calls, for which the host raises a single arithmetic
//!   error, `Error(Object, ArithDomain)`, identical for overflow, division by
//!   zero and `MIN / -1`.
//!
//! The `I256` phantom-overflow fallback runs on checked operations, so every
//! failure inside it, a zero `denominator` included, collapses into a single
//! `None` and surfaces as [`SorobanFixedPointError::Overflow`], consistent
//! with the one host error covering all these faults.
//!
//! The platform therefore never distinguishes a division by zero from an
//! overflow; a caller that needs a distinct division-by-zero signal has to
//! validate the denominator up front. That is what the higher-level helpers
//! such as [`wad::Wad::from_ratio`] do with
//! [`SorobanFixedPointError::DivisionByZero`].
//!
//! The checked variants return `None` for every failure above.
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

mod exp_ln;
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
    /// Base is outside the valid domain (e.g. `ln(x)` for `x <= 0`,
    /// or `powf(x, y)` with non-positive `x` combined with float exponent).
    InvalidBase = 1502,
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
