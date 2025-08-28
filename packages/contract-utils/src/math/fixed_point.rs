use soroban_sdk::{contracttype, Env};

use crate::math::soroban_fixed_point::SorobanFixedPoint;

#[contracttype]
pub enum Rounding {
    Floor, // Toward negative infinity
    Ceil,  // Toward positive infinity
}

/**
 * Calculates x * y / denominator with full precision, following the selected rounding direction.
 * Throws if result overflows a i128 or denominator is zero (handles phantom overflow).
 */
pub fn muldiv(e: &Env, x: i128, y: i128, denominator: i128, rounding: Rounding) -> i128 {
    match rounding {
        Rounding::Floor => x.fixed_mul_floor(e, &y, &denominator),
        Rounding::Ceil => x.fixed_mul_ceil(e, &y, &denominator),
    }
}
