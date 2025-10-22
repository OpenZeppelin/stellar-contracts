/*
MIT License

Copyright (c) 2023 Script3 Ltd. and contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

use soroban_sdk::{contracterror, Env};

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
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SorobanFixedPointError {
    /// The operation failed because the denominator is 0.
    ZeroDenominator = 1500,
    /// The operation failed because a phantom overflow occurred.
    PhantomOverflow = 1501,
    /// The operation failed because the result does not fit in Self.
    ResultOverflow = 1502,
}
