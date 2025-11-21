pub mod fixed_point;
mod i128_fixed_point;
mod i256_fixed_point;
mod soroban_fixed_point;
pub mod wad;

mod test;

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
