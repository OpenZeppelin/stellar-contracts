//! Grumpkin affine point arithmetic over Soroban's BN254 scalar field.
//!
//! Grumpkin is the prime-order elliptic curve `y² = x³ - 17` defined over
//! `F_r`, the scalar field of BN254. Because Grumpkin's base field equals
//! BN254's scalar field, every coordinate operation reduces to a `Bn254Fr`
//! host call (CAP-80: `bn254_fr_{add, sub, mul, inv}`).
//!
//! This module exposes:
//!
//! * [`Grumpkin::add`], [`Grumpkin::sub`], [`Grumpkin::neg`] — affine point
//!   arithmetic with full identity handling.
//! * [`Grumpkin::is_on_curve`], [`Grumpkin::is_not_identity`] — validation
//!   helpers for points that enter the system without a soundness proof.
//! * [`Grumpkin::identity`] — the encoded point at infinity, `(0, 0)` over 64
//!   bytes.
//!
//! # Encoding
//!
//! A [`Point`] is a `BytesN<64>` laid out as `be_bytes(x) || be_bytes(y)`, each
//! coordinate a canonical 32-byte big-endian `Bn254Fr` representative. The
//! identity is the all-zero encoding `(0, 0)` and is handled as a special case
//! in arithmetic. Real curve points cannot share this encoding because
//! `0² ≠ 0³ - 17 (mod r)`.

use soroban_sdk::{crypto::bn254::Bn254Fr, BytesN, Env, U256};

/// Affine encoding of a Grumpkin point as a 64-byte value.
///
/// Layout: `be_bytes(x) || be_bytes(y)`, each a canonical 32-byte `Bn254Fr`
/// representative. The all-zero encoding represents the identity (point at
/// infinity).
pub type Point = BytesN<64>;

/// Grumpkin point arithmetic over `Bn254Fr`.
///
/// The implementation performs no on-curve check on its inputs: callers must
/// ensure inputs are valid curve points before calling [`Self::add`],
/// [`Self::sub`], or [`Self::neg`]. Use [`Self::is_on_curve`] and
/// [`Self::is_not_identity`] at trust boundaries (e.g. user-supplied points
/// without an accompanying soundness proof).
pub struct Grumpkin;

impl Grumpkin {
    /// Returns the encoded identity element (the point at infinity).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn identity(e: &Env) -> Point {
        BytesN::from_array(e, &[0u8; 64])
    }

    /// Returns `true` iff `p` is the identity (all-zero 64-byte encoding).
    ///
    /// # Arguments
    ///
    /// * `p` - The point to check.
    pub fn is_identity(p: &Point) -> bool {
        p.to_array() == [0u8; 64]
    }

    /// Returns `true` iff `p` is not the identity.
    ///
    /// # Arguments
    ///
    /// * `p` - The point to check.
    pub fn is_not_identity(p: &Point) -> bool {
        !Self::is_identity(p)
    }

    /// Returns `true` iff `(x, y)` satisfies Grumpkin's curve equation
    /// `y² ≡ x³ - 17 (mod r)` **and** each coordinate is a canonical
    /// `Bn254Fr` representative (`< r` as a 32-byte big-endian integer).
    ///
    /// The canonical-range check is required for byte equality on validated
    /// points to be sound: without it, the same logical point `(x, y)` has
    /// multiple distinct encodings (e.g. `be(x) || be(y)` and
    /// `be(x + r) || be(y)`), all of which `Bn254Fr::from_bytes` reduces to
    /// the same value, breaking uniqueness checks keyed on the raw bytes.
    ///
    /// The identity encoding `(0, 0)` returns `false`: `0² ≠ -17 (mod r)`. To
    /// admit `O` as a valid group element, combine this with
    /// [`Self::is_identity`] at the call site.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `p` - The point to validate.
    pub fn is_on_curve(e: &Env, p: &Point) -> bool {
        let bytes = p.to_array();
        if !is_canonical_coord(&bytes[..32]) || !is_canonical_coord(&bytes[32..]) {
            return false;
        }
        let (x, y) = Self::coordinates(e, p);
        let lhs = y.clone() * y;
        let rhs = x.clone() * x.clone() * x - fr_from_u32(e, 17);
        lhs == rhs
    }

    /// Returns `-p`. For non-identity `p = (x, y)`, this is `(x, -y mod r)`;
    /// `-O = O`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `p` - The point to negate.
    pub fn neg(e: &Env, p: &Point) -> Point {
        if Self::is_identity(p) {
            return p.clone();
        }
        let (x, y) = Self::coordinates(e, p);
        let neg_y = fr_zero(e) - y;
        Self::from_xy(e, &x, &neg_y)
    }

    /// Returns `p1 + p2` on Grumpkin.
    ///
    /// Distinguishes the five cases of affine short-Weierstrass addition:
    ///
    /// 1. `p1 = O` → `p2`.
    /// 2. `p2 = O` → `p1`.
    /// 3. `x1 = x2` and `y1 = -y2 (mod r)` → `O` (inverse case). This must be
    ///    detected before the generic case, otherwise `x1 - x2 = 0` forces a
    ///    division by zero in the slope formula. It also subsumes the
    ///    two-torsion edge case `y = 0`, since then `y = -y` and `p = -p`.
    /// 4. `p1 = p2` → doubling with slope `λ_dbl = 3·x1² / (2·y1)`.
    /// 5. Otherwise (generic) → slope `λ_add = (y2 - y1) / (x2 - x1)`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `p1` - The first summand.
    /// * `p2` - The second summand.
    pub fn add(e: &Env, p1: &Point, p2: &Point) -> Point {
        if Self::is_identity(p1) {
            return p2.clone();
        }
        if Self::is_identity(p2) {
            return p1.clone();
        }

        let (x1, y1) = Self::coordinates(e, p1);
        let (x2, y2) = Self::coordinates(e, p2);

        if x1 == x2 {
            let neg_y2 = fr_zero(e) - y2.clone();
            if y1 == neg_y2 {
                return Self::identity(e);
            }
            // Doubling: y1 = y2 (and y1 ≠ 0, since y1 = -y1 ⇒ y1 = 0 is caught
            // above as the inverse case).
            let three = fr_from_u32(e, 3);
            let two = fr_from_u32(e, 2);
            let num = three * (x1.clone() * x1.clone());
            let denom = two * y1.clone();
            let lambda = num * denom.inv();
            // x3 = λ² - 2·x1
            let x3 = lambda.clone() * lambda.clone() - x1.clone() - x1.clone();
            // y3 = λ·(x1 - x3) - y1
            let y3 = lambda * (x1 - x3.clone()) - y1;
            return Self::from_xy(e, &x3, &y3);
        }

        // Generic case: distinct x-coordinates.
        let lambda = (y2 - y1.clone()) * (x2.clone() - x1.clone()).inv();
        let x3 = lambda.clone() * lambda.clone() - x1.clone() - x2;
        let y3 = lambda * (x1 - x3.clone()) - y1;
        Self::from_xy(e, &x3, &y3)
    }

    /// Returns `p1 - p2 = p1 + (-p2)`. Subtraction of a point from itself
    /// returns `O` via the inverse case in [`Self::add`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `p1` - The minuend.
    /// * `p2` - The subtrahend.
    pub fn sub(e: &Env, p1: &Point, p2: &Point) -> Point {
        Self::add(e, p1, &Self::neg(e, p2))
    }

    /// Encodes `(x, y)` as a 64-byte [`Point`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `x` - The x-coordinate as a `Bn254Fr` element.
    /// * `y` - The y-coordinate as a `Bn254Fr` element.
    pub fn from_xy(e: &Env, x: &Bn254Fr, y: &Bn254Fr) -> Point {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&x.to_bytes().to_array());
        bytes[32..].copy_from_slice(&y.to_bytes().to_array());
        BytesN::from_array(e, &bytes)
    }

    /// Decodes `(x, y)` from a 64-byte [`Point`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `p` - The encoded point to decode.
    pub fn coordinates(e: &Env, p: &Point) -> (Bn254Fr, Bn254Fr) {
        let bytes = p.to_array();
        let mut x = [0u8; 32];
        let mut y = [0u8; 32];
        x.copy_from_slice(&bytes[..32]);
        y.copy_from_slice(&bytes[32..]);
        (
            Bn254Fr::from_bytes(BytesN::from_array(e, &x)),
            Bn254Fr::from_bytes(BytesN::from_array(e, &y)),
        )
    }
}

fn fr_zero(e: &Env) -> Bn254Fr {
    Bn254Fr::from_u256(U256::from_u32(e, 0))
}

fn fr_from_u32(e: &Env, v: u32) -> Bn254Fr {
    Bn254Fr::from_u256(U256::from_u32(e, v))
}

/// BN254 scalar field order `r` in big-endian bytes — the canonical upper
/// bound on a coordinate's 32-byte representative.
const BN254_FR_MODULUS_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];

/// Returns `true` iff `coord` is a canonical 32-byte big-endian `Bn254Fr`
/// representative, i.e. lexicographically less than the field modulus `r`.
/// Lexicographic byte comparison on fixed-width 32-byte big-endian arrays
/// coincides with numeric comparison.
fn is_canonical_coord(coord: &[u8]) -> bool {
    coord < &BN254_FR_MODULUS_BE[..]
}
