// Based on the Soroban fixed-point mathematics library
// Original implementation: https://github.com/script3/soroban-fixed-point-math

// NOTE: Phantom overflow IS resolved here, without a wider intermediate type.
// When `x * y` overflows I256, both operands are split by the denominator and
// the division is distributed, which is an exact identity rather than an
// approximation:
//
//     x = q1*D + r1     y = q2*D + r2     (0 <= r1, r2 < D)
//     floor(x*y/D) = q1*q2*D + q1*r2 + r1*q2 + floor(r1*r2/D)

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
/// [`checked_mul_div_decomposed`] holds the reasoning: why the decomposition
/// works on absolute values, why they have to be unsigned, and why the
/// conversion goes through bytes.
fn to_magnitude(v: &I256) -> U256 {
    let e = v.env();
    let u = U256::from_be_bytes(e, &v.to_be_bytes());
    if *v < I256::from_i32(e, 0) {
        // Two's-complement negation, normally `!u + 1`, done without bitwise operations
        // because the SDK has none.
        //
        // Inverting every bit is the same as subtracting from all-ones. Every bit of
        // `U256::MAX` is 1, and at each position `1 - bit` yields the flipped bit, so
        // no position ever borrows from its neighbour and the subtraction is a
        // bit-for-bit inversion: `U256::MAX - u == !u` exactly. Therefore `-u
        // == (U256::MAX - u) + 1`.
        //
        // In an 8-bit analogue, negating 5: `255 - 5 + 1 == 251 == 0xFB`, and 0xFB read
        // as an i8 is -5.
        // Negating 128, which is `|i8::MIN|` and has no positive i8
        // form, gives `255 - 128 + 1 == 128 == 0x80`, which read as an i8 is
        // -128.
        // The same arithmetic at 256 bits is how `|I256::MIN|`, which is `2^255`,
        // survives the round trip.
        //
        // Neither the subtraction nor the addition can leave `U256` here: `v < 0`
        // reinterprets to `u >= 2^255`, so `U256::MAX - u <= 2^255 - 1`, and adding one
        // to that stays below `2^256`.
        U256::max_value(e).sub(&u).add(&U256::from_u32(e, 1))
    } else {
        u
    }
}

/// Converts a `U256` magnitude back to a signed `I256`, applying `negative`,
/// and returns `None` when the signed value does not fit.
///
/// The inverse of [`to_magnitude`], and the point where leaving `I256`'s range
/// is caught. Also returns `None`, rather than `Some(0)`, for a zero magnitude
/// with `negative` set; [`checked_mul_div_decomposed`] covers why that cannot
/// arise from the fallback and what it means for a test that calls the fallback
/// directly.
fn from_magnitude(e: &Env, mag: &U256, negative: bool) -> Option<I256> {
    if negative {
        // The largest magnitude any `I256` has is `2^255`, which sits well inside
        // `U256`. Coming back is different. `U256` reaches `2^256 - 1`, while `I256`
        // stops at `2^255 - 1` going up and `-2^255` going down, so most of the upper
        // half of `U256` has no signed counterpart. The magnitude arriving here was
        // produced by the summation, which is free to exceed the signed range, so it
        // has to be bounds-checked before it is reinterpreted.
        //
        // The limit differs by sign, because the signed range is not symmetric. A
        // negative result may use the whole `2^255`, since `I256::MIN` is `-2^255`.
        //
        // `from_parts` takes four 64-bit limbs, most significant first, so the constant
        // below is `0x8000_0000_0000_0000 * 2^192`, which is `2^63 * 2^192`, which is
        // `2^255`.
        //
        // The bound is compared explicitly rather than inferred from a sign mismatch
        // after reinterpretation. An explicit bound and a sign-mismatch check detect
        // the same condition, and sign-mismatch is one operation cheaper, but
        // the bound is what an auditor can
        // check against the type's documented range without reasoning about two's
        // complement.
        if *mag > U256::from_parts(e, 0x8000_0000_0000_0000, 0, 0, 0) {
            return None;
        }
        // The same `(U256::MAX - v) + 1` negation as in `to_magnitude`, run the other
        // way. `checked_add` rather than `add` is what handles a zero magnitude; see
        // the note on this function for why that case cannot arrive from the
        // fallback.
        let negated = U256::max_value(e).sub(mag).checked_add(&U256::from_u32(e, 1))?;
        // The bytes are unchanged; only their interpretation goes from unsigned to
        // signed.
        Some(I256::from_be_bytes(e, &negated.to_be_bytes()))
    } else {
        // The positive end stops one short of the negative end, at
        // `I256::MAX == 2^255 - 1`, so this bound is one lower than the one above. In
        // limbs, `(2^63 - 1) * 2^192` plus all-ones in the remaining three sums to
        // exactly `2^255 - 1`.
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
/// The fast path in each entry point multiplies first and divides second, so it
/// gives up whenever `x * y` leaves `I256`, even when `x * y / denominator`
/// would fit easily. This reaches the same answer without ever forming that
/// product.
///
/// The long path (phantom overflow path) is to divide before multiplying.
/// Splitting a value by the denominator gives a whole part and a leftover,
/// and every value has exactly one such split:
///
/// ```text
/// abs_x = q1 * abs_d + r1        with 0 <= r1 < abs_d
/// abs_y = q2 * abs_d + r2        with 0 <= r2 < abs_d
/// ```
///
/// The operands here are absolute values (not positive nor negative), with the
/// sign reapplied at the very end. Why they must be absolute is explained
/// further down, just after the identity; take it as given for a moment.
///
/// Substituting both splits into the product and expanding gives four terms:
///
/// ```text
/// abs_x * abs_y == (q1*abs_d + r1) * (q2*abs_d + r2)
///               == q1*q2*abs_d*abs_d + q1*r2*abs_d + r1*q2*abs_d + r1*r2
/// ```
///
/// Every term except `r1*r2` carries a factor of `abs_d`, so dividing the whole
/// line by `abs_d` cancels that factor exactly and leaves `r1*r2` as the only
/// term still under a division:
///
/// ```text
/// abs_x * abs_y / abs_d
///     == q1*q2*abs_d + q1*r2 + r1*q2 + rem_product / abs_d
/// ```
///
/// where `rem_product` is `r1 * r2`. That last line is the identity the
/// function computes, and its point is that the right-hand side never mentions
/// `abs_x * abs_y`. All four terms are built only from `q1`, `r1`, `q2`, `r2`
/// and `abs_d`.
///
/// A small case to fix the shape, `7 * 5 / 3`, whose answer is `11`:
///
/// ```text
/// 7 = 2*3 + 1   ->  q1 = 2, r1 = 1
/// 5 = 1*3 + 2   ->  q2 = 1, r2 = 2
///
/// q1*q2*3  +  q1*r2  +  r1*q2  +  (r1*r2)/3
///    6     +    4    +    1    +      0      == 11
/// ```
///
/// The two split equations, `abs_x = q1 * abs_d + r1` and `abs_y = q2 * abs_d +
/// r2`, have to hold exactly. If either is off, the expansion above starts from
/// a false premise and the four terms add up to the wrong number. That
/// requirement is what forces absolute values, because `q1` and `r1` come from
/// two separate SDK calls:
///
/// ```text
/// q1 = abs_x.div(abs_d)           truncates toward zero
/// r1 = abs_x.rem_euclid(abs_d)    never negative, always in 0 .. abs_d
/// ```
///
/// On a non-negative numerator `div` and `rem_euclid` agree, because truncating
/// and rounding down are the same thing there. On a negative numerator they do
/// not. Splitting `-7` by `3` is the smallest case that shows it, and there are
/// two ways to do it, both correct arithmetic:
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
/// remainder, `2`. Both calls are right on their own; they simply describe
/// different divisions, and the SDK offers no truncating `rem` that would
/// return the `-1` belonging to `-2`. Calling `div` and `rem_euclid` together
/// therefore pairs the truncating row's quotient with the rounding-down row's
/// remainder:
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
/// The magnitudes are unsigned rather than a signed `abs`, because a signed
/// absolute value does not always exist. `I256::MIN` is `-2^255` while
/// `I256::MAX` stops one short of `2^255`, so `|I256::MIN|` is larger than
/// anything the type holds, and `I256::MIN` does reach this function, for
/// example through `mul_div(I256::MIN, 3, 3)`. Converting in and
/// out goes through the big-endian byte encoding because the SDK offers no
/// other bridge between `I256` and `U256`: there is no `to_parts`, no bitwise
/// operations and no wrapping arithmetic. `exp_ln.rs` already relies on the
/// same round-trip, for the same kind of reason.
///
/// The terms `q1*q2*abs_d`, `q1*r2` and `r1*q2` are whole integers, so the
/// entire fractional part of the quotient sits in `rem_product / abs_d`. That
/// is why the truncated magnitude,
/// `mag`, is just those three terms plus `floor(rem_product / abs_d)`, and why
/// `rem_product % abs_d` alone decides whether the result is exact.
///
/// What the identity buys, beyond avoiding the product, is what bounds each of
/// its four terms. `q1*q2*abs_d` is at
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
/// One edge is worth knowing about. Reapplying a negative sign to a zero
/// magnitude is not representable by the negation [`from_magnitude`] uses, so
/// it answers `None` there. That cannot happen on any path a caller reaches:
/// arriving here at all requires `x * y` to have overflowed, so `|x * y| >=
/// 2^255`, and `|denominator| <= 2^255` for every `I256`, which
/// puts the quotient magnitude at one or above. It does happen when this
/// function is called directly with small operands, where the `None` reads as a
/// bug rather than as a domain violation. A test doing that has to stay inside
/// `|x * y| >= |denominator|`, which is the domain production actually reaches.
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

    // `q1*q2*abs_d`, `q1*r2` and `r1*q2` are integers, so the entire fractional
    // part of `|x * y| / abs_d` is carried by `rem_product / abs_d`. Exactness
    // is decided there alone, and a non-zero `rem_product` does not imply an
    // inexact result.
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
