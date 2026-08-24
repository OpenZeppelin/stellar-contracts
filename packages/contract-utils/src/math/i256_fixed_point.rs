// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: Phantom overflow IS resolved here, without a wider intermediate type.
// When `x * y` overflows I256, both operands are split by the denominator and
// the division is distributed, which is an exact identity rather than an
// approximation:
//
//     x = q1*D + r1     y = q2*D + r2     (0 <= r1, r2 < D)
//     floor(x*y/D) = q1*q2*D + q1*r2 + r1*q2 + floor(r1*r2/D)
//
// Three of the four terms are bounded by the answer or by an input, so they fit
// whenever the inputs and the result do. Only `r1*r2` is bounded by D alone,
// which yields the single condition `|denominator| <= 2^128`. Inside that
// domain the fallback returns bit-for-bit what a 512-bit intermediate would;
// outside it the `r1 * r2` multiplication is checked, so the operation rejects
// rather than returning an incorrect value.

use soroban_sdk::{panic_with_error, Env, I256, U256};

use crate::math::{Rounding, SorobanFixedPointError};

/// Calculates `x * y / denominator` following the specified rounding direction.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
///
/// # Errors
///
/// * refer to the errors of [`mul_div`]
pub fn mul_div_with_rounding(x: I256, y: I256, denominator: I256, rounding: Rounding) -> I256 {
    match rounding {
        Rounding::Floor => mul_div_floor(&x, &y, &denominator),
        Rounding::Ceil => mul_div_ceil(&x, &y, &denominator),
        Rounding::Truncate => mul_div(&x, &y, &denominator),
    }
}

/// Checked version of [`mul_div_with_rounding`].
///
/// Calculates `x * y / denominator`, returning `None` instead of panicking when
/// the result cannot be represented in `I256`, when `denominator` is zero, or
/// when the intermediate `x * y` overflows `I256` and `denominator` is too
/// large for the fallback to recover it.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The second operand.
/// * `denominator` - The divisor.
/// * `rounding` - The rounding direction to use.
pub fn checked_mul_div_with_rounding(
    x: I256,
    y: I256,
    denominator: I256,
    rounding: Rounding,
) -> Option<I256> {
    match rounding {
        Rounding::Floor => checked_mul_div_floor(&x, &y, &denominator),
        Rounding::Ceil => checked_mul_div_ceil(&x, &y, &denominator),
        Rounding::Truncate => checked_mul_div(&x, &y, &denominator),
    }
}

/// Calculates floor(x * y / denominator).
///
/// When the intermediate `x * y` overflows `I256`, the result is recovered by
/// remainder decomposition instead of failing; refer to the module
/// documentation for the domain.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * refer to the errors of [`mul_div`]
pub fn mul_div_floor(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let e = x.env();
    match x.checked_mul(y) {
        Some(r) => div_floor(&r, denominator),
        // Reaching `None` means either the true result does not fit `I256`, or
        // `|denominator|` is above the `2^128` ceiling described in the module docs.
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Floor)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)),
    }
}

/// Calculates ceil(x * y / denominator).
///
/// When the intermediate `x * y` overflows `I256`, the result is recovered by
/// remainder decomposition instead of failing; refer to the module
/// documentation for the domain.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * refer to the errors of [`mul_div`]
pub fn mul_div_ceil(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let e = x.env();
    match x.checked_mul(y) {
        Some(r) => div_ceil(&r, denominator),
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Ceil)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)),
    }
}

/// Calculates `x * y / denominator` (truncated toward zero).
///
/// When the intermediate `x * y` overflows `I256`, the result is recovered by
/// remainder decomposition instead of failing; refer to the module
/// documentation for the domain.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
///
/// # Errors
///
/// * [`SorobanFixedPointError::Overflow`] - when `x * y` overflows `I256` and
///   the result still cannot be recovered, either because it does not fit
///   `I256` or because `|denominator|` exceeds `2^128`.
///
/// # Notes
///
/// Domain errors are left to the host rather than mapped to a contract error,
/// since this is a plain arithmetic operation. A zero `denominator` and
/// `I256::MIN / -1` both fail with the host's own arithmetic error.
/// [`checked_mul_div`] returns `None` for both.
pub fn mul_div(x: &I256, y: &I256, denominator: &I256) -> I256 {
    let e = x.env();
    match x.checked_mul(y) {
        Some(r) => r.div(denominator),
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Truncate)
            .unwrap_or_else(|| panic_with_error!(e, SorobanFixedPointError::Overflow)),
    }
}

/// Calculates floor(x * y / denominator).
///
/// Returns `None` under the same conditions as [`checked_mul_div`].
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_floor(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    match x.checked_mul(y) {
        Some(r) => checked_div_floor(&r, denominator),
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Floor),
    }
}

/// Calculates ceil(x * y / denominator).
///
/// Returns `None` under the same conditions as [`checked_mul_div`].
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div_ceil(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    match x.checked_mul(y) {
        Some(r) => checked_div_ceil(&r, denominator),
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Ceil),
    }
}

/// Calculates `x * y / denominator` (truncated toward zero).
///
/// Returns `None` if `denominator` is zero, if the result does not fit `I256`,
/// or if `x * y` overflows `I256` and `|denominator|` exceeds `2^128`. An
/// intermediate `x * y` that overflows `I256` is no longer a failure by itself;
/// refer to the module documentation for the domain.
///
/// # Arguments
///
/// * `x` - The first operand.
/// * `y` - The multiplicand.
/// * `denominator` - The divisor.
pub fn checked_mul_div(x: &I256, y: &I256, denominator: &I256) -> Option<I256> {
    match x.checked_mul(y) {
        Some(r) => r.checked_div(denominator),
        None => checked_mul_div_decomposed(x, y, denominator, Rounding::Truncate),
    }
}

// ###################### HELPERS ######################

/// Returns the absolute value of `v` as an unsigned `U256` magnitude.
///
/// # Notes
///
/// [`checked_mul_div_decomposed`] explains why the decomposition needs absolute
/// values at all. What forces them to be *unsigned* is narrower: `|I256::MIN|`
/// is `2^255` while `I256::MAX` stops one short of it, so a signed absolute
/// value does not always exist, and `I256::MIN` does reach the fallback, for
/// example `mul_div(I256::MIN, 3, 3)`.
///
/// The conversion goes through the big-endian byte encoding because the SDK
/// offers no other bridge between `I256` and `U256`: there is no `to_parts`, no
/// bitwise operations and no wrapping arithmetic. `exp_ln.rs` already relies on
/// the same round-trip for the same kind of reason, a value that fits unsigned
/// but not signed.
fn to_magnitude(v: &I256) -> U256 {
    let e = v.env();
    let u = U256::from_be_bytes(e, &v.to_be_bytes());
    if *v < I256::from_i32(e, 0) {
        // Two's-complement negation without bitwise operations, since the SDK exposes
        // none: `!u + 1 == (U256::MAX - u) + 1`. A negative `v` reinterprets to
        // `u >= 2^255`, so `U256::MAX - u <= 2^255 - 1` and neither step can
        // leave the type.
        U256::max_value(e).sub(&u).add(&U256::from_u32(e, 1))
    } else {
        u
    }
}

/// Converts a `U256` magnitude back to a signed `I256`, applying `negative`,
/// and returns `None` when the signed value does not fit.
///
/// This is the inverse of [`to_magnitude`] and the point where leaving `I256`'s
/// range is detected. Magnitude arithmetic ranges over the whole of `U256`, but
/// only magnitudes up to `2^255` are representable when the result is negative,
/// and one less than that when it is non-negative.
///
/// The bound is compared explicitly against the type's documented range rather
/// than inferred from a sign mismatch after reinterpretation. Both detect the
/// same condition and the sign-mismatch form is one operation cheaper, but the
/// explicit bound is what an auditor can check without reasoning about two's
/// complement.
///
/// # Notes
///
/// This helper returns `None`, not `Some(0)`, for a zero magnitude with
/// `negative` set, because negating zero would carry past `2^256`. That is
/// sound only because the sole caller cannot produce a zero magnitude: entering
/// the fallback requires `|x * y| >= 2^255`, and `|denominator| <= 2^255` holds
/// for every `I256`, so the quotient magnitude is at least one. A future caller
/// that can reach zero must handle the sign itself.
fn from_magnitude(e: &Env, mag: &U256, negative: bool) -> Option<I256> {
    if negative {
        // `-I256::MIN` is `2^255`, so a negative result may use the full magnitude.
        if *mag > U256::from_parts(e, 0x8000_0000_0000_0000, 0, 0, 0) {
            return None;
        }
        // `checked_add` rather than `add`: negating a zero magnitude would carry past
        // `2^256`. The note above explains why a zero magnitude cannot reach here.
        let negated = U256::max_value(e).sub(mag).checked_add(&U256::from_u32(e, 1))?;
        Some(I256::from_be_bytes(e, &negated.to_be_bytes()))
    } else {
        // `I256::MAX` is `2^255 - 1`.
        if *mag > U256::from_parts(e, 0x7fff_ffff_ffff_ffff, u64::MAX, u64::MAX, u64::MAX) {
            return None;
        }
        Some(I256::from_be_bytes(e, &mag.to_be_bytes()))
    }
}

/// Computes `x * y / denominator` by remainder decomposition.
///
/// Returns `None` when `denominator` is zero, when the result does not fit
/// `I256`, or when `|denominator|` is large enough that the `r1 * r2` term
/// overflows `U256`.
///
/// # Notes
///
/// Everything below works on absolute values (`abs_x`, `abs_y`, `abs_d`), with
/// the sign reapplied at the end; [`to_magnitude`] explains why absolute values
/// are needed. Dividing each operand by the denominator gives a quotient and a
/// remainder:
///
/// ```text
/// abs_x = q1 * abs_d + r1        with 0 <= r1 < abs_d
/// abs_y = q2 * abs_d + r2        with 0 <= r2 < abs_d
/// ```
///
/// Substituting both into `abs_x * abs_y / abs_d` and expanding gives an exact
/// identity, writing `rem_product` for `r1 * r2`:
///
/// ```text
/// abs_x * abs_y / abs_d
///     == q1*q2*abs_d + q1*r2 + r1*q2 + rem_product / abs_d
/// ```
///
/// Both of those substitutions have to be exactly true, or the identity is
/// derived from a false premise, and that is what forces absolute values. The
/// two halves of each split come from separate SDK calls:
///
/// ```text
/// q1 = abs_x.div(abs_d)           truncates toward zero
/// r1 = abs_x.rem_euclid(abs_d)    never negative, always in 0 .. abs_d
/// ```
///
/// On non-negative input those agree, because truncating and rounding down are
/// the same thing there. On a negative numerator they do not. Splitting `-7` by
/// `3` is the smallest case that shows it, and there are two ways to do it,
/// both correct arithmetic:
///
/// ```text
/// truncate toward zero:  -7 / 3 = -2.33 -> -2      -7 = (-2 * 3) + (-1)
///                                                  quotient -2, remainder -1
///
/// round down:            -7 / 3 = -2.33 -> -3      -7 = (-3 * 3) + 2
///                                                  quotient -3, remainder 2
/// ```
///
/// `div` truncates, so it drops the fraction of `-2.33` and returns the first
/// row's quotient, `-2`. `rem_euclid` is defined never to return a negative
/// value, whatever the signs of its operands, so it reports the second row's
/// remainder, `2`. Each call is right on its own. They just describe different
/// divisions, and the SDK offers no truncating `rem` that would return the `-1`
/// belonging to `-2`. Calling both therefore pairs a quotient from one row with
/// a remainder from the other:
///
/// ```text
/// q1 * abs_d + r1  ==  (-2 * 3) + 2  ==  -4        but the numerator was -7
/// ```
///
/// Off by exactly one `abs_d`. A false split does not raise an error, it
/// silently changes the answer. For `x = -7`, `y = 5`, `denominator = 3`, where
/// the true truncated result is `-11`:
///
/// ```text
/// signed split:    q1 = -2, r1 = 2, q2 = 1, r2 = 2
///                  terms  -6 + (-4) + 2 + 1  =  -7       wrong, and returned as success
///
/// magnitudes:      q1 =  2, r1 = 1, q2 = 1, r2 = 2
///                  terms    6 +  4  + 1 + 0  =  11       sign reapplied  ->  -11
/// ```
///
/// Signed operands are not impossible here, but they would have to derive the
/// remainder themselves as `x - q * d` rather than call `rem_euclid`.
///
/// The first three terms are whole integers, so the entire fractional part of
/// the quotient sits in the last one. That is why the truncated magnitude,
/// `mag`, is just those three terms plus `floor(rem_product / abs_d)`, and why
/// `rem_product % abs_d` alone decides whether the result is exact.
///
/// The point of the rearrangement is what bounds each term. `q1*q2*abs_d` is at
/// most the answer itself, `q1*r2` is at most `abs_x`, and `r1*q2` is at most
/// `abs_y`, so all three fit whenever the inputs and the answer do. Only
/// `rem_product` is bounded by the denominator alone, at just under `abs_d *
/// abs_d`, with no relation to how big the answer is. That single term is the
/// whole source of the `|denominator| <= 2^128` domain, since `abs_d * abs_d`
/// has to stay below `2^256`. It is also the one place the unsigned range earns
/// its keep: capped at `I256` instead, the ceiling would fall to `2^127.5`.
/// With `abs_d` at `2^128 - 1` and both remainders maximal, `rem_product`
/// reaches `2^256 - 2^130 + 4`, twice what `I256` holds and comfortably inside
/// `U256`.
///
/// `pub(super)` rather than private so that the differential test comparing
/// this fallback against the fast path, on the domain where both are defined,
/// can reach it from `crate::math::test`. `pub(super)` resolves to `pub(in
/// crate::math)`, which covers the test module as a descendant;
/// `exp_ln::ln_wad` and `exp_ln::exp_wad` use the same visibility.
pub(super) fn checked_mul_div_decomposed(
    x: &I256,
    y: &I256,
    denominator: &I256,
    rounding: Rounding,
) -> Option<I256> {
    let e = x.env();

    let abs_x = to_magnitude(x);
    let abs_y = to_magnitude(y);
    let abs_d = to_magnitude(denominator);

    // Checked rather than plain division: `abs_d` is zero exactly when
    // `denominator` is, and returning `None` for that is what keeps the checked
    // entry points total. The unchecked ones only reach this function when `x *
    // y` overflowed; a zero denominator on the fast path is left to the host.
    let q1 = abs_x.checked_div(&abs_d)?;
    let r1 = abs_x.checked_rem_euclid(&abs_d)?;
    let q2 = abs_y.checked_div(&abs_d)?;
    let r2 = abs_y.checked_rem_euclid(&abs_d)?;

    // `r1 * r2` is the only term bounded by `abs_d` alone rather than by an input
    // or by the answer, which makes this multiplication the exact detector for
    // `|denominator| > 2^128`: it can fail only when `abs_d * abs_d` exceeds
    // `2^256`, so nothing inside the documented domain reaches the failure and
    // nothing outside it receives a wrong answer.
    let rem_product = r1.checked_mul(&r2)?;

    // Every term is non-negative, so each partial sum is bounded by the final
    // magnitude. Summation order is therefore irrelevant to overflow: a failure
    // here always means the true result is unrepresentable, never that the
    // terms were added in an unlucky order.
    let mut mag = q1.checked_mul(&q2)?.checked_mul(&abs_d)?;
    mag = mag.checked_add(&q1.checked_mul(&r2)?)?;
    mag = mag.checked_add(&r1.checked_mul(&q2)?)?;
    mag = mag.checked_add(&rem_product.checked_div(&abs_d)?)?;

    // The first three terms are integers, so the entire fractional part of
    // `|x * y| / |abs_d|` is carried by `rem_product / abs_d`. Exactness is decided
    // there alone, and a non-zero `rem_product` does not imply an inexact
    // result.
    let exact = rem_product.checked_rem_euclid(&abs_d)? == U256::from_u32(e, 0);

    // The result is negative iff an odd number of the three operands are. That
    // matches the sign of the true result only because this function is
    // unreachable with a zero operand: a product involving zero never
    // overflows, so it stays on the fast path.
    let zero = I256::from_i32(e, 0);
    let negative = (*x < zero) ^ (*y < zero) ^ (*denominator < zero);

    // `mag` is the truncated magnitude, so in magnitude space all three modes
    // reduce to one question: whether to round away from zero. Floor moves away
    // from zero only for a negative result, Ceil only for a positive one, and
    // Truncate never does.
    let round_away_from_zero = match rounding {
        Rounding::Truncate => false,
        Rounding::Floor => negative && !exact,
        Rounding::Ceil => !negative && !exact,
    };
    if round_away_from_zero {
        mag = mag.checked_add(&U256::from_u32(e, 1))?;
    }

    from_magnitude(e, &mag, negative)
}

/// Performs checked floor(r / z)
fn checked_div_floor(r: &I256, z: &I256) -> Option<I256> {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);

    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.checked_rem_euclid(z)?;
        let one = I256::from_i32(env, 1);
        let quotient = r.checked_div(z)?;
        quotient.checked_sub(if remainder > *zero { &one } else { zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.checked_div(z)
    }
}

/// Performs floor(r / z)
fn div_floor(r: &I256, z: &I256) -> I256 {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);
    if (r < zero && z > zero) || (r > zero && z < zero) {
        // ceil is taken by default for a negative result
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).sub(if remainder > *zero { &one } else { zero })
    } else {
        // floor is taken by default for a positive or zero result
        r.div(z)
    }
}

/// Performs checked ceil(r / z)
fn checked_div_ceil(r: &I256, z: &I256) -> Option<I256> {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);

    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        r.checked_div(z)
    } else {
        // floor is taken by default for a positive result
        let remainder = r.checked_rem_euclid(z)?;
        let one = I256::from_i32(env, 1);
        let quotient = r.checked_div(z)?;
        quotient.checked_add(if remainder > *zero { &one } else { zero })
    }
}

/// Performs ceil(r / z)
fn div_ceil(r: &I256, z: &I256) -> I256 {
    let env = r.env();
    let zero = &I256::from_i32(env, 0);
    if (r <= zero && z > zero) || (r >= zero && z < zero) {
        // ceil is taken by default for a negative or zero result
        r.div(z)
    } else {
        let remainder = r.rem_euclid(z);
        let one = I256::from_i32(env, 1);
        r.div(z).add(if remainder > *zero { &one } else { zero })
    }
}
