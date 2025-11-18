use core::{
    cmp::{Ord, PartialOrd},
    ops::{Add, Div, Mul, Neg, Sub},
};

use soroban_sdk::{contracterror, panic_with_error, Env};

/// Fixed-point decimal with 18 decimal places (WAD precision)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wad(i128);

pub const WAD_SCALE: i128 = 1_000_000_000_000_000_000;

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WadError {
    /// Arithmetic overflow occurred
    Overflow = 1600,
    /// Division by zero
    DivisionByZero = 1601,
    /// Invalid decimal conversion (token_decimals too large)
    InvalidDecimals = 1602,
}

fn pow10(exp: u32) -> i128 {
    10_i128.pow(exp)
}

impl Wad {
    /// Creates a Wad from an integer by applying WAD scaling.
    ///
    /// Treats the input as a whole number and scales it to WAD precision (18
    /// decimals).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `n` - The integer value to convert to WAD representation.
    ///
    /// # Errors
    ///
    /// * [`WadError::Overflow`] - When the multiplication overflows i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_integer(&e, 5);
    /// assert_eq!(wad.raw(), 5_000_000_000_000_000_000);
    /// ```
    ///
    /// # Notes
    ///
    /// Compare with [`Wad::from_raw`] which does NOT apply WAD scaling.
    pub fn from_integer(e: &Env, n: i128) -> Self {
        Wad(n.checked_mul(WAD_SCALE).unwrap_or_else(|| panic_with_error!(e, WadError::Overflow)))
    }

    /// Converts Wad back to an integer by removing WAD scaling.
    ///
    /// Truncates toward zero, discarding any fractional part.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_raw(5_000_000_000_000_000_000);
    /// assert_eq!(wad.to_integer(), 5);
    /// ```
    pub fn to_integer(self) -> i128 {
        self.0 / WAD_SCALE
    }

    /// Creates a Wad from a ratio (num / den).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `num` - The numerator of the ratio.
    /// * `den` - The denominator of the ratio.
    ///
    /// # Errors
    ///
    /// * [`WadError::DivisionByZero`] - When `den` is zero.
    /// * [`WadError::Overflow`] - When the multiplication overflows i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_ratio(&e, 5, 10);
    /// assert_eq!(wad.raw(), 500_000_000_000_000_000); // 0.5 in WAD
    /// ```
    pub fn from_ratio(e: &Env, num: i128, den: i128) -> Self {
        if den == 0 {
            panic_with_error!(e, WadError::DivisionByZero)
        }
        let scaled =
            num.checked_mul(WAD_SCALE).unwrap_or_else(|| panic_with_error!(e, WadError::Overflow));
        Wad(scaled / den)
    }

    /// Creates a Wad from a token amount with specified decimals.
    ///
    /// Converts a token's native representation to WAD (18 decimals).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `amount` - The token amount in its smallest unit.
    /// * `token_decimals` - The number of decimals the token uses.
    ///
    /// # Errors
    ///
    /// * [`WadError::Overflow`] - When the scaling multiplication overflows
    ///   i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // USDC has 2 decimals, so 1 USDC = 100 units
    /// let wad = Wad::from_token_amount(&e, 100, 2);
    /// assert_eq!(wad.raw(), 1_000_000_000_000_000_000); // 1.0 in WAD
    /// ```
    ///
    /// # Notes
    ///
    /// `amount` must be in the token's smallest unit. For example, to represent
    /// 1 USDC (2 decimals), pass `100`, not `1`.
    pub fn from_token_amount(e: &Env, amount: i128, token_decimals: u8) -> Self {
        if token_decimals == 18 {
            Wad(amount)
        } else if token_decimals < 18 {
            let diff = 18u32 - token_decimals as u32;
            let factor = pow10(diff);
            Wad(amount
                .checked_mul(factor)
                .unwrap_or_else(|| panic_with_error!(e, WadError::Overflow)))
        } else {
            let diff = token_decimals as u32 - 18u32;
            let factor = pow10(diff);
            Wad(amount / factor)
        }
    }

    /// Converts Wad to a token amount with specified decimals.
    ///
    /// Converts from WAD (18 decimals) back to a token's native representation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_decimals` - The number of decimals the target token uses.
    ///
    /// # Errors
    ///
    /// * [`WadError::Overflow`] - When the scaling multiplication overflows
    ///   i128 (occurs when `token_decimals > 18`).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_raw(1_000_000_000_000_000_000); // 1.0 in WAD
    /// let usdc_amount = wad.to_token_amount(&e, 2);
    /// assert_eq!(usdc_amount, 100); // 1 USDC = 100 units
    /// ```
    pub fn to_token_amount(self, e: &Env, token_decimals: u8) -> i128 {
        if token_decimals == 18 {
            self.0
        } else if token_decimals < 18 {
            let diff = 18u32 - token_decimals as u32;
            let factor = pow10(diff);
            self.0 / factor
        } else {
            let diff = token_decimals as u32 - 18u32;
            let factor = pow10(diff);
            self.0.checked_mul(factor).unwrap_or_else(|| panic_with_error!(e, WadError::Overflow))
        }
    }

    /// Creates a Wad from a price with specified decimals.
    ///
    /// This is an alias for [`Wad::from_token_amount`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `price_integer` - The price in its smallest unit.
    /// * `price_decimals` - The number of decimals the price uses.
    ///
    /// # Errors
    ///
    /// refer to [`Wad::from_token_amount`] errors.
    pub fn from_price(e: &Env, price_integer: i128, price_decimals: u8) -> Self {
        Wad::from_token_amount(e, price_integer, price_decimals)
    }

    /// Returns the raw i128 value without applying WAD scaling.
    ///
    /// Returns the internal representation directly.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_integer(5);
    /// assert_eq!(wad.raw(), 5_000_000_000_000_000_000);
    /// ```
    pub fn raw(self) -> i128 {
        self.0
    }

    /// Creates a Wad from a raw i128 value without applying WAD scaling.
    ///
    /// Interprets the input as the internal representation directly.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw internal value.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_raw(5);
    /// // Represents 0.000000000000000005 in decimal
    /// assert_eq!(wad.raw(), 5);
    /// ```
    ///
    /// # Notes
    ///
    /// Compare with [`Wad::from_integer`] which applies WAD scaling.
    pub fn from_raw(raw: i128) -> Self {
        Wad(raw)
    }

    /// Returns the minimum of two Wad values.
    ///
    /// # Arguments
    ///
    /// * `other` - The other Wad value to compare.
    pub fn min(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }

    /// Returns the maximum of two Wad values.
    ///
    /// # Arguments
    ///
    /// * `other` - The other Wad value to compare.
    pub fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }

    // ################## CHECKED ARITHMETIC ##################

    /// Checked addition. Returns `None` on overflow.
    pub fn checked_add(self, rhs: Wad) -> Option<Wad> {
        self.0.checked_add(rhs.0).map(Wad)
    }

    /// Checked subtraction. Returns `None` on overflow.
    pub fn checked_sub(self, rhs: Wad) -> Option<Wad> {
        self.0.checked_sub(rhs.0).map(Wad)
    }

    /// Checked multiplication (Wad * Wad). Returns `None` on overflow.
    pub fn checked_mul(self, rhs: Wad) -> Option<Wad> {
        self.0.checked_mul(rhs.0).map(|product| Wad(product / WAD_SCALE))
    }

    /// Checked division (Wad / Wad). Returns `None` on overflow or division by
    /// zero.
    pub fn checked_div(self, rhs: Wad) -> Option<Wad> {
        if rhs.0 == 0 {
            return None;
        }
        self.0.checked_mul(WAD_SCALE).map(|scaled| Wad(scaled / rhs.0))
    }

    /// Checked multiplication by integer. Returns `None` on overflow.
    pub fn checked_mul_int(self, n: i128) -> Option<Wad> {
        self.0.checked_mul(n).map(Wad)
    }

    /// Checked division by integer. Returns `None` on division by zero.
    pub fn checked_div_int(self, n: i128) -> Option<Wad> {
        if n == 0 {
            return None;
        }
        Some(Wad(self.0 / n))
    }
}

// Wad + Wad
impl Add for Wad {
    type Output = Wad;

    fn add(self, rhs: Wad) -> Wad {
        Wad(self.0 + rhs.0)
    }
}

// Wad - Wad
impl Sub for Wad {
    type Output = Wad;

    fn sub(self, rhs: Wad) -> Wad {
        Wad(self.0 - rhs.0)
    }
}

// Wad * Wad: fixed-point multiplication (a * b) / WAD_SCALE
impl Mul for Wad {
    type Output = Wad;

    fn mul(self, rhs: Wad) -> Wad {
        Wad((self.0 * rhs.0) / WAD_SCALE)
    }
}

// Wad / Wad: fixed-point division (a * WAD_SCALE) / b
impl Div for Wad {
    type Output = Wad;

    fn div(self, rhs: Wad) -> Wad {
        Wad((self.0 * WAD_SCALE) / rhs.0)
    }
}

// Negation
impl Neg for Wad {
    type Output = Wad;

    fn neg(self) -> Wad {
        Wad(-self.0)
    }
}

// Wad * i128: multiply by integer (no WAD scaling)
impl Mul<i128> for Wad {
    type Output = Wad;

    fn mul(self, rhs: i128) -> Wad {
        Wad(self.0 * rhs)
    }
}

// i128 * Wad: multiply by integer (no WAD scaling)
impl Mul<Wad> for i128 {
    type Output = Wad;

    fn mul(self, rhs: Wad) -> Wad {
        Wad(self * rhs.0)
    }
}

// Wad / i128: divide by integer (no WAD scaling)
impl Div<i128> for Wad {
    type Output = Wad;

    fn div(self, rhs: i128) -> Wad {
        Wad(self.0 / rhs)
    }
}

// ============================================================================
// Design Decision: Why we DON'T implement From<i128> / Into<i128>
// ============================================================================
//
// ```
// impl From<i32> for Wad {
//     fn from(n: i32) -> Self {
//         // `Wad::from_integer(n)` or `Wad::from_raw(n)`?
//     }
// }
// ```
// ============================================================================
//
// The `From<i128>` trait is intentionally NOT implemented because the
// conversion semantics are fundamentally ambiguous. There are two equally valid
// interpretations:
//
// 1. Scaled conversion (semantic interpretation): `Wad::from(5)` could mean
//    "the number 5.0" → calls `from_integer(5)` → internal value:
//    5_000_000_000_000_000_000
//
// 2. Unscaled conversion (raw value interpretation): `Wad::from(5)` could mean
//    "5 raw units" → calls `from_raw(5)` → internal value: 5 (represents
//    0.000000000000000005)
//
// Both interpretations are valid and useful in different contexts. Without
// explicit context, it's impossible to determine which one the user intends.
// This ambiguity can lead to critical bugs in financial calculations.
//
// Instead, we require explicit method calls:
// - Use `Wad::from_integer(n)` when you mean "the number n" (will WAD scale the
//   input)
// - Use `Wad::from_raw(n)` when you mean "n raw units" (will NOT WAD scale the
//   input)
//
// This design follows Rust API guidelines: conversions should be obvious and
// unambiguous. When multiple reasonable interpretations exist, use named
// constructors instead of trait implementations.
