//! Natural logarithm and exponential for `Wad` (18-decimal fixed-point).
//!
//! "Exponential" here means the **natural exponential function** `e^x`,
//! where `e ≈ 2.71828` is Euler's number. It is *not* a generic power
//! function — `x^y` for arbitrary base `x` is implemented as
//! [`crate::math::wad::Wad::powf`], which composes `exp` and `ln` via
//! `x^y = exp(y * ln(x))`.
//!
//! Ported from Solmate's `SignedWadMath.sol`:
//! <https://github.com/transmissions11/solmate/blob/main/src/utils/SignedWadMath.sol>
//!
//! Solmate's algorithm uses `int256` throughout because:
//! - Polynomial coefficients are pre-scaled by 2^96 for precision; intermediate
//!   products routinely exceed 2^192.
//! - The final scaling factors in `ln` are themselves > 2^192.
//!
//! This module mirrors that algorithm using soroban-sdk `I256`. Inputs and
//! outputs are `i128` in WAD (10^18) scale; everything in between is `I256`.
//! `to_i128()` at the end naturally returns `None` if the result doesn't fit.
//!
//! The "magic" decimal constants are minimax coefficients of `(p, q)` Padé
//! approximants of `ln(m)` on `m ∈ [1, 2)` and `exp(r)` on
//! `r ∈ [-ln(2)/2, ln(2)/2)`, both pre-scaled by 2^96.

use soroban_sdk::{Env, I256, U256};

/// Input threshold at/below which `exp_wad(x)` returns 0.
///
/// For `x` this small, `e^x * 10^18` evaluates to less than `0.5`, which
/// truncates to `0` in WAD precision. The exact value is
/// `floor(ln(0.5 * 10^-18) * 10^18)` ≈ `-42.139 * 10^18`.
const EXP_INPUT_MIN: i128 = -42_139_678_854_452_767_551;

/// Input threshold at/above which `exp_wad(x)` overflows.
///
/// For `x` this large, `e^x` exceeds the range Solmate's algorithm can
/// represent in its internal i256-with-2^96-scale basis. The exact value
/// is `floor(ln((2^255 - 1) / 10^18) * 10^18)` ≈ `135.305 * 10^18`.
const EXP_INPUT_MAX: i128 = 135_305_999_368_893_231_589;

/// Natural logarithm of a WAD value.
///
/// Returns `None` if `x <= 0` or the result doesn't fit in `i128`.
/// Output is in WAD scale (i.e. `ln(x) * 10^18`).
pub(super) fn ln_wad(e: &Env, x: i128) -> Option<i128> {
    if x <= 0 {
        return None;
    }

    // r = floor(log2(x)). Solmate uses an inline bit-scan; for an `i128`
    // input we get the same result from Rust's `ilog2`.
    let r = x.ilog2() as i32; // 0 ..= 126
    let k = r - 96; // -96 ..= 30

    // From here on, all arithmetic is in I256.
    let mut x = I256::from_i128(e, x);

    // Solmate normalizes x to (1, 2) * 2^96 via:
    //     x <<= 159 - k; x = (x as uint256) >> 159;
    // Rewriting without relying on i256 wraparound: shift left when k is
    // negative, shift right when k is positive.
    if k < 0 {
        x = x.shl((-k) as u32);
    } else if k > 0 {
        x = x.shr(k as u32);
    }

    // ----- (8, 8)-term rational approximation of ln(x) on [1, 2) -----
    // Coefficients are pre-scaled by 2^96.

    // p = x + 3273285459638523848632254066296
    let mut p = x.add(&I256::from_parts(e, 0, 0, 0x29508e4585, 0x43d8aa4df2abee78));
    // p = ((p * x) >> 96) + 24828157081833163892658089445524
    p = p.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x139601a2efa, 0xbe717e604cbb4894));
    // p = ((p * x) >> 96) + 43456485725739037958740375743393
    p = p.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x2247f7a7b65, 0x94320649aa03aba1));
    // p = ((p * x) >> 96) - 11111509109440967052023855526967
    p = p.mul(&x).shr(96).sub(&I256::from_parts(e, 0, 0, 0x8c3f38e95a, 0x6b1ff2ab1c3b3437));
    // p = ((p * x) >> 96) - 45023709667254063763336534515857
    p = p.mul(&x).shr(96).sub(&I256::from_parts(e, 0, 0, 0x2384773bdf1, 0xac5676facced6091));
    // p = ((p * x) >> 96) - 14706773417378608786704636184526
    p = p.mul(&x).shr(96).sub(&I256::from_parts(e, 0, 0, 0xb9a025d814, 0xb29c212b8b1a07ce));
    // p = p * x - (795164235651350426258249787498 << 96)
    // (Pre-shifted constant; leaves p in 2^192 basis to skip a final shift.)
    p = p.mul(&x).sub(&I256::from_parts(e, 0xa, 0x09507084cc699bb0, 0xe71ea86a00000000, 0));

    // q polynomial — q has no zeros in the domain (all roots complex).

    // q = x + 5573035233440673466300451813936
    let mut q = x.add(&I256::from_parts(e, 0, 0, 0x465772b2bb, 0xbb5f824b15207a30));
    // q = ((q * x) >> 96) + 71694874799317883764090561454958
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x388eaa27412, 0xd5aca026815d636e));
    // q = ((q * x) >> 96) + 283447036172924575727196451306956
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0xdf99ac50203, 0x1bf953eff472fdcc));
    // q = ((q * x) >> 96) + 401686690394027663651624208769553
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x13cdffb29d51, 0xd99322bdff5f2211));
    // q = ((q * x) >> 96) + 204048457590392012362485061816622
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0xa0f742023de, 0xf783a307a986912e));
    // q = ((q * x) >> 96) + 31853899698501571402653359427138
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x1920d8043ca, 0x89b5239253284e42));
    // q = ((q * x) >> 96) + 909429971244387300277376558375
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0xb7a86d737, 0x5468fac667a0a527));

    // r = p / q  -- result in (0, 0.125) * 2^96 basis (still 2^192 because we
    // skipped the final shr in the p polynomial).
    let mut r = p.div(&q);

    // Final scaling. Solmate folds three steps into one accumulator:
    //   * multiply by scale factor s (≈ 5.549) * 5e18 * 2^96
    //   * add k * ln(2) * 5e18 * 2^192
    //   * add ln(2^96 / 10^18) * 5e18 * 2^192
    //   * shr 174 to convert from 5^18 * 2^192 basis back to 10^18 (WAD).

    // r *= 1677202110996718588342820967067443963516166
    r = r.mul(&I256::from_parts(e, 0, 0x1340, 0xdaa0d5f769dba191, 0x5cef59f0815a5506));
    // r += 16597577552685614221487285958193947469193820559219878177908093499208371
    // * k
    let k_i256 = I256::from_i32(e, k);
    r = r.add(
        &I256::from_parts(
            e,
            0x267a36c0c95,
            0xb3975ab3ee5b203a,
            0x7614a3f75373f047,
            0xd803ae7b6687f2b3,
        )
        .mul(&k_i256),
    );
    // r += 600920179829731861736702779321621459595472258049074101567377883020018308
    r = r.add(&I256::from_parts(
        e,
        0x57115e47018c,
        0x7177eebf7cd370a3,
        0x356a1b7863008a5a,
        0xe8028c72b8864284,
    ));
    // r >>= 174  (5^18 * 2^192 → 10^18)
    r = r.shr(174);

    r.to_i128()
}

/// Natural exponential of a WAD value: returns `e^x` where `e ≈ 2.71828`
/// is Euler's number.
///
/// This is **not** a power function. To raise an arbitrary base to an
/// arbitrary power, use [`crate::math::wad::Wad::powf`].
///
/// Returns `Some(0)` for `x <= EXP_INPUT_MIN` (underflow to 0, matching
/// Solmate). Returns `None` for `x >= EXP_INPUT_MAX` (would overflow even
/// in i256-with-2^96-scale).
pub(super) fn exp_wad(e: &Env, x: i128) -> Option<i128> {
    if x <= EXP_INPUT_MIN {
        return Some(0);
    }
    if x >= EXP_INPUT_MAX {
        return None;
    }

    // Convert from 10^18 to 2^96 fixed-point. The factor is
    // 1e18 / 2^96 = 5^18 / 2^78, so `x = (x << 78) / 5^18`. The shifted
    // value can exceed i128, so do this in I256.
    let x = I256::from_i128(e, x).shl(78).div(&I256::from_i128(e, 3_814_697_265_625)); // 5^18

    // Range-reduce: pick integer k so that r = x - k*ln(2)*2^96 has
    // |r| ≤ ln(2)/2 * 2^96. Round-to-nearest is `(x + ln2_2pow96 / 2) / ln2_2pow96`,
    // rewritten as Solmate does for asm efficiency:
    //   k = ((x << 96) / ln2_2pow96 + 2^95) >> 96
    let ln2_2pow96 = I256::from_parts(e, 0, 0, 0xb17217f7, 0xd1cf79abc9e3b398); // 54916777467707473351141471128
    let two_pow_95 = I256::from_i32(e, 1).shl(95);
    let k = x.shl(96).div(&ln2_2pow96).add(&two_pow_95).shr(96);
    let x = x.sub(&k.mul(&ln2_2pow96));

    // ----- (6, 7)-term rational approximation of exp(r) -----

    // y = x + 1346386616545796478920950773328
    let y_initial = x.add(&I256::from_parts(e, 0, 0, 0x10fe68e7fd, 0x37d0007b713f7650));
    // y = ((y * x) >> 96) + 57155421227552351082224309758442
    let y = y_initial.mul(&x).shr(96).add(&I256::from_parts(
        e,
        0,
        0,
        0x2d16720577b,
        0xd19bf614176fe9ea,
    ));
    // p = y + x - 94201549194550492254356042504812
    let mut p = y.add(&x).sub(&I256::from_parts(e, 0, 0, 0x4a4fd9f2a8b, 0x96949216d2255a6c));
    // p = ((p * y) >> 96) + 28719021644029726153956944680412240
    p = p.mul(&y).shr(96).add(&I256::from_parts(e, 0, 0, 0x587f503bb6ea2, 0x9d25fcb740196450));
    // p = p * x + (4385272521454847904659076985693276 << 96)
    // (pre-shifted constant)
    p = p.mul(&x).add(&I256::from_parts(e, 0xd835, 0xebba824c98fb31b8, 0x3b2ca45c00000000, 0));

    // q = x - 2855989394907223263936484059900
    let mut q = x.sub(&I256::from_parts(e, 0, 0, 0x240c330e9f, 0xb2d9cbaf0fd5aafc));
    // q = ((q * x) >> 96) + 50020603652535783019961831881945
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x277594991cf, 0xc85f6e2461837cd9));
    // q = ((q * x) >> 96) - 533845033583426703283633433725380
    q = q.mul(&x).shr(96).sub(&I256::from_parts(e, 0, 0, 0x1a521255e34f, 0x6a5061b25ef1c9c4));
    // q = ((q * x) >> 96) + 3604857256930695427073651918091429
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0xb1bbb201f443, 0xcf962f1a1d3db4a5));
    // q = ((q * x) >> 96) - 14423608567350463180887372962807573
    q = q.mul(&x).shr(96).sub(&I256::from_parts(e, 0, 0, 0x2c72388d9f74f, 0x51a9331fed693f15));
    // q = ((q * x) >> 96) + 26449188498355588339934803723976023
    q = q.mul(&x).shr(96).add(&I256::from_parts(e, 0, 0, 0x5180bb14799ab, 0x47a8a8cb2a527d57));

    // r = p / q  -- result in (0.09, 0.25) * 2^96 (still 2^192 because we
    // skipped a final shr in the p polynomial).
    let r = p.div(&q);

    // Combined final scaling. Multiply by:
    //   * scale factor s (≈ 6.031...) * 1e18 * 2^96
    //   * 2^k from range reduction
    //   * 1e18 / 2^96 base conversion
    // Done as a single multiplication by `exp_final_mul` followed by a
    // shr of `(195 - k)`.
    //
    // Solmate's source casts `r` to `uint256` here:
    //     int256((uint256(r) * 3822833...) >> uint256(195 - k))
    // This matters because the product `r * final_mul` can reach ~2^255,
    // which overflows signed 256-bit but fits unsigned. We mirror that by
    // moving the multiplication and shift into U256 (the final result is
    // always positive — `exp` is positive — so the cast back is safe).
    let r_bytes = r.to_be_bytes();
    let r_u256 = U256::from_be_bytes(e, &r_bytes);

    // dec: 3822833074963236453042738258902158003155416615667
    let final_mul = U256::from_parts(e, 0, 0x29d9dc385, 0x63c32e5c2f6dc192, 0xee70ef65f9978af3);

    // k is i32-sized after range reduction; narrow via to_i128.
    let k_i128 = k.to_i128()?;
    let shr_amount = (195_i128 - k_i128) as u32;

    let result_bytes = r_u256.mul(&final_mul).shr(shr_amount).to_be_bytes();
    I256::from_be_bytes(e, &result_bytes).to_i128()
}
