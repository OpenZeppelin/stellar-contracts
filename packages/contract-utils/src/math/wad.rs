use core::{
    cmp::{Ord, PartialOrd},
    ops::{Add, Div, Mul, Neg, Sub},
};

use soroban_sdk::{panic_with_error, Env};

use crate::math::{
    exp_ln::{exp_wad, ln_wad},
    i128_fixed_point::checked_mul_div,
    SorobanFixedPointError,
};

/// Fixed-point decimal number with 18 decimal places of precision.
///
/// `Wad` represents decimal numbers using a fixed-point representation where
/// 1.0 is stored as `1_000_000_000_000_000_000` (10^18). This provides precise
/// decimal arithmetic suitable for financial calculations in smart contracts.
///
/// # Truncation
///
/// All arithmetic operations truncate toward zero rather than rounding:
/// - `5 / 2 = 2` (not 2.5 or 3)
/// - `-5 / 2 = -2` (not -2.5 or -3)
///
/// ## Precision
///
/// Due to truncation on each multiplication/division, the order of operations
/// can affect results:
///
/// ```ignore
/// let a = Wad::from_integer(&e, 1000);
/// let b = Wad::from_raw(55_000_000_000_000_000);  // 0.055
/// let c = Wad::from_raw(8_333_333_333_333_333);   // ~0.00833
///
/// let result1 = a * b * c;      // Truncates after first multiplication
/// let result2 = a * (b * c);    // Truncates after inner multiplication
/// // result1 and result2 may differ by ~10^-16 due to different truncation points
/// ```
///
/// **Typical precision loss:** ~10^-15 to 10^-16 in relative terms, which is
/// negligible when converting to typical token precision (6-8 decimals).
///
/// # Examples
///
/// ```ignore
/// use soroban_sdk::Env;
/// use contract_utils::math::wad::Wad;
///
/// let e = Env::default();
///
/// // Creating Wad values
/// let five = Wad::from_integer(&e, 5);           // 5.0
/// let half = Wad::from_ratio(&e, 1, 2);          // 0.5
/// let price = Wad::from_token_amount(&e, 1_500_000, 6); // 1.5 (from USDC)
///
/// // Arithmetic
/// let sum = five + half;                          // 5.5
/// let product = five * half;                      // 2.5
/// let quotient = five / half;                     // 10.0
///
/// // Converting back to token amounts
/// let usdc_amount = product.to_token_amount(&e, 6); // 2_500_000 (2.5 USDC)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wad(i128);

pub const WAD_SCALE: i128 = 1_000_000_000_000_000_000;

fn pow10(e: &Env, exp: u32) -> i128 {
    if exp > 38 {
        panic_with_error!(e, SorobanFixedPointError::Overflow);
    }
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
    /// * [`SorobanFixedPointError::Overflow`] - When the multiplication
    ///   overflows i128.
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
        Wad(n
            .checked_mul(WAD_SCALE)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)))
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
    /// * [`SorobanFixedPointError::DivisionByZero`] - When `den` is zero.
    /// * [`SorobanFixedPointError::Overflow`] - When the multiplication
    ///   overflows i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let wad = Wad::from_ratio(&e, 5, 10);
    /// assert_eq!(wad.raw(), 500_000_000_000_000_000); // 0.5 in WAD
    /// ```
    pub fn from_ratio(e: &Env, num: i128, den: i128) -> Self {
        if den == 0 {
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero)
        }
        checked_mul_div(e, &num, &WAD_SCALE, &den)
            .map(Wad)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Creates a Wad from a token amount with specified decimals.
    ///
    /// Converts a token's native representation to WAD (18 decimals).
    /// Truncates toward zero when scaling down (token_decimals > 18).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `amount` - The token amount in its smallest unit.
    /// * `token_decimals` - The number of decimals the token uses.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::Overflow`] - When the scaling multiplication
    ///   overflows i128.
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
            let factor = pow10(e, diff);
            Wad(amount
                .checked_mul(factor)
                .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)))
        } else {
            let diff = token_decimals as u32 - 18u32;
            let factor = pow10(e, diff);
            Wad(amount / factor)
        }
    }

    /// Converts Wad to a token amount with specified decimals.
    ///
    /// Converts from WAD (18 decimals) back to a token's native representation.
    /// Truncates toward zero when scaling down (token_decimals < 18).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_decimals` - The number of decimals the target token uses.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::Overflow`] - When the scaling multiplication
    ///   overflows i128 (occurs when `token_decimals > 18`).
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
            let factor = pow10(e, diff);
            self.0 / factor
        } else {
            let diff = token_decimals as u32 - 18u32;
            let factor = pow10(e, diff);
            self.0
                .checked_mul(factor)
                .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
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

    /// Checked multiplication (Wad * Wad).
    ///
    /// Returns `None` on overflow. Handles phantom overflow by scaling to
    /// `I256` when intermediate multiplication overflows `i128` but the final
    /// result fits. Result is truncated toward zero after division by
    /// `WAD_SCALE`.
    pub fn checked_mul(self, e: &Env, rhs: Wad) -> Option<Wad> {
        checked_mul_div(e, &self.0, &rhs.0, &WAD_SCALE).map(Wad)
    }

    /// Checked division (Wad / Wad). Returns `None` on overflow or division by
    /// zero.
    ///
    /// Result is truncated toward zero.
    pub fn checked_div(self, e: &Env, rhs: Wad) -> Option<Wad> {
        if rhs.0 == 0 {
            return None;
        }
        checked_mul_div(e, &self.0, &WAD_SCALE, &rhs.0).map(Wad)
    }

    /// Checked multiplication by integer. Returns `None` on overflow.
    pub fn checked_mul_int(self, n: i128) -> Option<Wad> {
        self.0.checked_mul(n).map(Wad)
    }

    /// Checked division by integer. Returns `None` on division by zero or on
    /// overflow (`i128::MIN / -1`).
    pub fn checked_div_int(self, n: i128) -> Option<Wad> {
        self.0.checked_div(n).map(Wad)
    }

    /// Returns the absolute value of the Wad.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let negative = Wad::from_integer(&e, -5);
    /// assert_eq!(negative.abs(), Wad::from_integer(&e, 5));
    /// ```
    pub fn abs(self) -> Self {
        Wad(self.0.abs())
    }

    /// Raises Wad to an unsigned integer power using exponentiation by
    /// squaring.
    ///
    /// This method is optimized for efficiency, computing the result in O(log
    /// n) multiplications where n is the exponent. Each multiplication
    /// maintains fixed-point precision by dividing by WAD_SCALE, with
    /// truncation toward zero.
    ///
    /// For non-integer exponents, see [`Wad::powf`] (uses `exp(y * ln(x))`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment for error handling.
    /// * `exponent` - The unsigned integer exponent (0 to 2^32-1).
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::Overflow`] - When intermediate or final
    ///   result exceeds i128 bounds.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Compound interest: (1.05)^10
    /// let rate = Wad::from_ratio(&e, 105, 100);  // 1.05
    /// let final_multiplier = rate.powi(&e, 10);
    /// let final_amount = principal * final_multiplier;
    ///
    /// // Quadratic bonding curve: price = supply^2
    /// let supply = Wad::from_integer(&e, 1000);
    /// let price = supply.powi(&e, 2);
    /// ```
    pub fn powi(self, e: &Env, exponent: u32) -> Self {
        self.checked_powi(e, exponent)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Checked version of [`Wad::powi`].
    ///
    /// Returns `None` instead of panicking on overflow. Handles phantom
    /// overflow transparently by scaling to `I256` when intermediate
    /// multiplications overflow `i128` but the final result fits.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment for i256 operations.
    /// * `exponent` - The unsigned integer exponent.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let small = Wad::from_integer(&e, 2);
    /// assert_eq!(small.checked_powi(&e, 10), Some(Wad::from_integer(&e, 1024)));
    ///
    /// let large = Wad::from_integer(&e, i128::MAX / WAD_SCALE);
    /// assert_eq!(large.checked_powi(&e, 2), None); // Overflows
    /// ```
    ///
    /// # Notes
    ///
    /// Phantom overflow is handled internally.
    pub fn checked_powi(self, e: &Env, mut exponent: u32) -> Option<Self> {
        // Handle base cases
        if exponent == 0 {
            return Some(Wad(WAD_SCALE)); // x^0 = 1
        }

        if exponent == 1 {
            return Some(self);
        }

        if self.0 == 0 {
            return Some(Wad::from_raw(0)); // 0^n = 0
        }

        if self.0 == WAD_SCALE {
            return Some(self); // 1^n = 1
        }

        // Exponentiation by squaring - processes exponent bit-by-bit
        let mut base = self;
        let mut result = Wad(WAD_SCALE); // Start with 1 in WAD

        // Example: x^10, where 10 in binary = 1010₂
        //
        // Binary:  1    0    1    0
        //          ↓    ↓    ↓    ↓
        // Powers:  x^8  x^4  x^2  x^1
        //          │    │    │    │
        // Bit=1?   Y    N    Y    N
        //          │    │    │    │
        // Action:  MUL  ---  MUL  ---  (only multiply result when bit=1)
        //          SQR  SQR  SQR  ---  (always square base for next)
        //
        // Result: x^8 * x^2 = x^10
        //
        // Note: We use checked_mul_div to handle phantom overflow
        // (where intermediate multiplication overflows i128 but final result fits).
        // This automatically scales to i256 when needed and returns None if the
        // result doesn't fit in i128.
        while exponent > 0 {
            if exponent & 1 == 1 {
                // result = result * base (in fixed-point)
                let new_result = checked_mul_div(e, &result.0, &base.0, &WAD_SCALE)?;
                result = Wad(new_result);
            }

            exponent >>= 1;
            if exponent > 0 {
                // base = base * base (in fixed-point)
                let new_base = checked_mul_div(e, &base.0, &base.0, &WAD_SCALE)?;
                base = Wad(new_base);
            }
        }

        Some(result)
    }

    /// Natural logarithm of `self`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::InvalidBase`] - When `self <= 0`. `ln` is
    ///   undefined for non-positive inputs.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let one = Wad::from_integer(&e, 1);
    /// assert_eq!(one.ln(&e), Wad::from_raw(0)); // ln(1) = 0
    /// ```
    pub fn ln(self, e: &Env) -> Self {
        self.checked_ln(e)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::InvalidBase))
    }

    /// Checked version of [`Wad::ln`].
    ///
    /// Returns `None` if `self <= 0`.
    pub fn checked_ln(self, e: &Env) -> Option<Self> {
        ln_wad(e, self.0).map(Wad)
    }

    /// Natural exponential of `self`: returns `e^self`, where `e ≈ 2.71828`
    /// is Euler's number.
    ///
    /// This is **not** a generic power function — for `x^y` with an arbitrary
    /// base, use [`Wad::powf`]. `Wad::exp` is the inverse of [`Wad::ln`]
    ///
    /// Returns `0` for inputs at or below `≈ -42.139` (the result rounds to 0
    /// in WAD precision). Panics for inputs at or above `≈ 135.305` (overflow).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::Overflow`] - When the input is too large to
    ///   represent the result in i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let zero = Wad::from_raw(0);
    /// assert_eq!(zero.exp(&e), Wad::from_integer(&e, 1)); // exp(0) = 1
    /// ```
    pub fn exp(self, e: &Env) -> Self {
        self.checked_exp(e)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Checked version of [`Wad::exp`]. Computes `e^self` where `e` is
    /// Euler's number; not a power function.
    ///
    /// Returns `None` if the input is at or above the overflow bound.
    pub fn checked_exp(self, e: &Env) -> Option<Self> {
        exp_wad(e, self.0).map(Wad)
    }

    /// Raises `self` to a `Wad` power (the floating-point analogue of
    /// [`Wad::powi`]).
    ///
    /// Computes `self^exponent` for arbitrary fractional exponents using
    /// `exp(exponent * ln(self))`. Falls back to integer exponentiation
    /// (via [`Wad::powi`]) when `exponent` is an exact non-negative integer
    /// in the `u32` range — this is faster and more precise.
    ///
    /// # Special cases
    ///
    /// - `x^0 = 1` for any `x` (including `0^0 = 1`, matching the convention
    ///   used by [`Wad::powi`], Python's `pow`, and IEEE 754).
    /// - `0^y = 0` for `y > 0`.
    /// - `1^y = 1` for any `y`.
    /// - `x^1 = x` for any `x`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `exponent` - The exponent.
    ///
    /// # Errors
    ///
    /// * [`SorobanFixedPointError::DivisionByZero`] - When `self == 0` and
    ///   `exponent < 0`.
    /// * [`SorobanFixedPointError::InvalidBase`] - When `self < 0`
    /// * [`SorobanFixedPointError::Overflow`] - When the result doesn't fit in
    ///   i128.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // 2^0.5 ≈ √2
    /// let two = Wad::from_integer(&e, 2);
    /// let half = Wad::from_ratio(&e, 1, 2);
    /// let sqrt_two = two.powf(&e, half);
    /// ```
    pub fn powf(self, e: &Env, exponent: Wad) -> Self {
        // 0^0 = 1
        if exponent.0 == 0 {
            return Wad(WAD_SCALE);
        }
        // 0^y for y > 0 = 0;
        // 0^y for y < 0 → division by zero
        if self.0 == 0 {
            if exponent.0 > 0 {
                return Wad(0);
            }
            panic_with_error!(e, SorobanFixedPointError::DivisionByZero);
        }
        // 1^y = 1
        if self.0 == WAD_SCALE {
            return self;
        }
        // x^1 = x
        if exponent.0 == WAD_SCALE {
            return self;
        }

        // Integer-y fast path: when `exponent` is an exact non-negative
        // integer that fits in u32, delegate to the integer `powi`. This
        // also handles `x < 0` correctly (e.g. `(-2)^3 = -8`), which the
        // general path below rejects (`(-2)^1.5` is complex).
        //
        // `exponent.0 % WAD_SCALE == 0` checks for "no fractional part":
        // a WAD value is an exact integer iff its raw representation is a
        // clean multiple of `WAD_SCALE` (10^18). For example, `2.0` has
        // raw = `2 * 10^18` (mod = 0); `2.5` has raw = `2.5 * 10^18`
        // (mod ≠ 0).
        if exponent.0 > 0 && exponent.0 % WAD_SCALE == 0 {
            let n = exponent.0 / WAD_SCALE;
            if n <= u32::MAX as i128 {
                return self.powi(e, n as u32);
            }
        }

        // General path requires a positive base (because we compute ln(x)).
        if self.0 < 0 {
            panic_with_error!(e, SorobanFixedPointError::InvalidBase);
        }

        // x^y = exp(y * ln(x))
        let ln_x = ln_wad(e, self.0)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::InvalidBase));
        let y_ln_x = checked_mul_div(e, &exponent.0, &ln_x, &WAD_SCALE)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow));
        exp_wad(e, y_ln_x)
            .map(Wad)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow))
    }

    /// Checked version of [`Wad::powf`].
    ///
    /// Special cases mirror [`Wad::powf`]: `x^0 = 1` for any `x` (including
    /// `0^0 = 1`), `0^y = 0` for `y > 0`, `1^y = 1`, `x^1 = x`.
    ///
    /// Returns `None` for all error conditions: overflow, `0^negative`,
    /// and negative base with non-integer exponent.
    pub fn checked_powf(self, e: &Env, exponent: Wad) -> Option<Self> {
        if exponent.0 == 0 {
            return Some(Wad(WAD_SCALE));
        }
        if self.0 == 0 {
            return if exponent.0 > 0 { Some(Wad(0)) } else { None };
        }
        if self.0 == WAD_SCALE {
            return Some(self);
        }
        if exponent.0 == WAD_SCALE {
            return Some(self);
        }

        // Integer-y fast path. `exponent.0 % WAD_SCALE == 0` means the WAD
        // value has no fractional part (raw is an exact multiple of 10^18).
        // See `powf` for details.
        if exponent.0 > 0 && exponent.0 % WAD_SCALE == 0 {
            let n = exponent.0 / WAD_SCALE;
            if n <= u32::MAX as i128 {
                return self.checked_powi(e, n as u32);
            }
        }

        if self.0 < 0 {
            return None;
        }

        // x^y = exp(y * ln(x))
        let ln_x = ln_wad(e, self.0)?;
        let y_ln_x = checked_mul_div(e, &exponent.0, &ln_x, &WAD_SCALE)?;
        exp_wad(e, y_ln_x).map(Wad)
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
// Result is truncated toward zero.
impl Mul for Wad {
    type Output = Wad;

    fn mul(self, rhs: Wad) -> Wad {
        Wad((self.0 * rhs.0) / WAD_SCALE)
    }
}

// Wad / Wad: fixed-point division (a * WAD_SCALE) / b
// Result is truncated toward zero.
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
// explicit context, it's impossible to determine which interpretation is
// intended.
// This ambiguity can lead to critical bugs in financial calculations.
//
// Instead, we require explicit method calls:
// - Use `Wad::from_integer(n)` for the interpretation "the number n" (the input
//   will be WAD-scaled)
// - Use `Wad::from_raw(n)` for the interpretation "n raw units" (the input will
//   NOT be WAD-scaled)
//
// This design follows Rust API guidelines: conversions should be obvious and
// unambiguous. When multiple reasonable interpretations exist, use named
// constructors instead of trait implementations.
