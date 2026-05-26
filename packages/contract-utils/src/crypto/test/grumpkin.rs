//! Tests for the Grumpkin point arithmetic module.
//!
//! Ground-truth point values are produced by `ark-grumpkin` (off-host) and
//! piped through the 64-byte `(be(x) || be(y))` encoding into the on-host
//! arithmetic. The property tests cover the algebraic identities of an
//! abelian group — associativity, commutativity, identity, inversion — plus
//! Pedersen homomorphism, and exercise the case-by-case branches of `add`
//! (left/right identity, inverse, doubling, generic).

extern crate std;

use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use ark_ff::{BigInteger, PrimeField, Zero};
use ark_grumpkin::{Affine as ArkPoint, Fr as ArkScalar, Projective as ArkProj};
use proptest::prelude::*;
use soroban_sdk::{BytesN, Env};

use crate::crypto::grumpkin::{Grumpkin, Point};

// ################## HELPERS ##################

/// Encodes an `ark-grumpkin` affine point as the 64-byte on-chain `Point`
/// (`be(x) || be(y)`, identity = 64 zero bytes).
fn to_point(e: &Env, p: &ArkPoint) -> Point {
    if p.is_zero() {
        return Grumpkin::identity(e);
    }
    let mut bytes = [0u8; 64];
    let x = p.x().expect("non-identity has x").into_bigint().to_bytes_be();
    let y = p.y().expect("non-identity has y").into_bigint().to_bytes_be();
    // ark BigInt::to_bytes_be() returns a fixed N*8-byte buffer; for Grumpkin's
    // 254-bit base field N = 4 so the output is always 32 bytes.
    bytes[..32].copy_from_slice(&x);
    bytes[32..].copy_from_slice(&y);
    BytesN::from_array(e, &bytes)
}

fn scalar_mul_g(k: ArkScalar) -> ArkPoint {
    (ArkProj::generator() * k).into_affine()
}

fn scalar_from_bytes(seed: &[u8]) -> ArkScalar {
    ArkScalar::from_le_bytes_mod_order(seed)
}

fn rand_point(e: &Env, seed: &[u8; 32]) -> Point {
    to_point(e, &scalar_mul_g(scalar_from_bytes(seed)))
}

/// A second Grumpkin generator `H`, distinct from `G`, used for Pedersen
/// commitments in the homomorphism test. Independence from `G` is not required
/// for the algebraic identity `commit(v1, r1) + commit(v2, r2) =
/// commit(v1+v2, r1+r2)`; any non-identity point works.
fn h_generator() -> ArkPoint {
    // H = 2 * G — fixed, deterministic, non-identity.
    scalar_mul_g(ArkScalar::from(2u64))
}

fn pedersen_commit(v: ArkScalar, r: ArkScalar) -> ArkPoint {
    (ArkProj::generator() * v + h_generator() * r).into_affine()
}

// ################## UNIT TESTS — case-by-case branches ##################

#[test]
fn identity_is_all_zero_encoding() {
    let e = Env::default();
    let o = Grumpkin::identity(&e);
    assert_eq!(o.to_array(), [0u8; 64]);
    assert!(Grumpkin::is_identity(&o));
    assert!(!Grumpkin::is_not_identity(&o));
}

#[test]
fn left_identity_returns_other_operand() {
    let e = Env::default();
    let p = rand_point(&e, &[0x11; 32]);
    let o = Grumpkin::identity(&e);
    assert_eq!(Grumpkin::add(&e, &o, &p).to_array(), p.to_array());
}

#[test]
fn right_identity_returns_other_operand() {
    let e = Env::default();
    let p = rand_point(&e, &[0x22; 32]);
    let o = Grumpkin::identity(&e);
    assert_eq!(Grumpkin::add(&e, &p, &o).to_array(), p.to_array());
}

#[test]
fn inverse_branch_returns_identity() {
    let e = Env::default();
    let p = rand_point(&e, &[0x33; 32]);
    let neg_p = Grumpkin::neg(&e, &p);
    assert_eq!(Grumpkin::add(&e, &p, &neg_p).to_array(), [0u8; 64]);
    assert_eq!(Grumpkin::add(&e, &neg_p, &p).to_array(), [0u8; 64]);
}

#[test]
fn doubling_branch_matches_reference() {
    let e = Env::default();
    let k = ArkScalar::from(7u64);
    let p_ark = scalar_mul_g(k);
    let two_p_ark = scalar_mul_g(k + k);

    let p = to_point(&e, &p_ark);
    let two_p = to_point(&e, &two_p_ark);

    assert_eq!(Grumpkin::add(&e, &p, &p).to_array(), two_p.to_array());
}

#[test]
fn generic_branch_matches_reference() {
    let e = Env::default();
    let k1 = ArkScalar::from(3u64);
    let k2 = ArkScalar::from(11u64);

    let p1 = to_point(&e, &scalar_mul_g(k1));
    let p2 = to_point(&e, &scalar_mul_g(k2));
    let expected = to_point(&e, &scalar_mul_g(k1 + k2));

    assert_eq!(Grumpkin::add(&e, &p1, &p2).to_array(), expected.to_array());
}

#[test]
fn neg_identity_is_identity() {
    let e = Env::default();
    let o = Grumpkin::identity(&e);
    assert_eq!(Grumpkin::neg(&e, &o).to_array(), [0u8; 64]);
}

#[test]
fn neg_matches_reference() {
    let e = Env::default();
    let k = ArkScalar::from(5u64);
    let p_ark = scalar_mul_g(k);
    let neg_p_ark = (-p_ark.into_group()).into_affine();

    let p = to_point(&e, &p_ark);
    let expected = to_point(&e, &neg_p_ark);

    assert_eq!(Grumpkin::neg(&e, &p).to_array(), expected.to_array());
}

#[test]
fn sub_self_is_identity() {
    let e = Env::default();
    let p = rand_point(&e, &[0x55; 32]);
    assert_eq!(Grumpkin::sub(&e, &p, &p).to_array(), [0u8; 64]);
}

#[test]
fn sub_matches_add_with_neg() {
    let e = Env::default();
    let p1 = rand_point(&e, &[0x66; 32]);
    let p2 = rand_point(&e, &[0x77; 32]);
    let lhs = Grumpkin::sub(&e, &p1, &p2);
    let rhs = Grumpkin::add(&e, &p1, &Grumpkin::neg(&e, &p2));
    assert_eq!(lhs.to_array(), rhs.to_array());
}

// ################## UNIT TESTS — on-curve / identity validation
// ##################

#[test]
fn is_on_curve_accepts_real_points() {
    let e = Env::default();
    let p = rand_point(&e, &[0x99; 32]);
    assert!(Grumpkin::is_on_curve(&e, &p));
}

#[test]
fn is_on_curve_rejects_identity() {
    let e = Env::default();
    // (0, 0) is the identity encoding; it does not satisfy y² = x³ − 17.
    let o = Grumpkin::identity(&e);
    assert!(!Grumpkin::is_on_curve(&e, &o));
}

#[test]
fn is_on_curve_rejects_off_curve_point() {
    let e = Env::default();
    // (1, 1) — 1 ≠ 1 - 17 (mod r), so not on the curve.
    let mut bytes = [0u8; 64];
    bytes[31] = 1;
    bytes[63] = 1;
    let p = BytesN::from_array(&e, &bytes);
    assert!(!Grumpkin::is_on_curve(&e, &p));
}

#[test]
fn is_on_curve_rejects_corrupted_y() {
    let e = Env::default();
    let mut p = rand_point(&e, &[0xAA; 32]).to_array();
    // Flip a bit in y.
    p[40] ^= 0x01;
    let p = BytesN::from_array(&e, &p);
    assert!(!Grumpkin::is_on_curve(&e, &p));
}

#[test]
fn is_not_identity_distinguishes_zero_from_real() {
    let e = Env::default();
    let o = Grumpkin::identity(&e);
    let p = rand_point(&e, &[0xBB; 32]);
    assert!(!Grumpkin::is_not_identity(&o));
    assert!(Grumpkin::is_not_identity(&p));
}

#[test]
fn from_xy_and_coordinates_roundtrip() {
    let e = Env::default();
    let p = rand_point(&e, &[0xCC; 32]);
    let (x, y) = Grumpkin::coordinates(&e, &p);
    let rebuilt = Grumpkin::from_xy(&e, &x, &y);
    assert_eq!(rebuilt.to_array(), p.to_array());
}

// ################## PROPERTY TESTS ##################

fn nonzero_scalar(seed: &[u8; 32]) -> ArkScalar {
    let mut s = scalar_from_bytes(seed);
    // Avoid scalar = 0 because then k * G = O and many identities collapse;
    // we cover the identity case explicitly in unit tests.
    if s.is_zero() {
        s = ArkScalar::from(1u64);
    }
    s
}

#[test]
fn prop_commutativity() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k1: [u8; 32], k2: [u8; 32])| {
        let s1 = nonzero_scalar(&k1);
        let s2 = nonzero_scalar(&k2);
        let p1 = to_point(&e, &scalar_mul_g(s1));
        let p2 = to_point(&e, &scalar_mul_g(s2));

        let lhs = Grumpkin::add(&e, &p1, &p2);
        let rhs = Grumpkin::add(&e, &p2, &p1);
        prop_assert_eq!(lhs.to_array(), rhs.to_array());
    })
}

#[test]
fn prop_associativity() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k1: [u8; 32], k2: [u8; 32], k3: [u8; 32])| {
        let s1 = nonzero_scalar(&k1);
        let s2 = nonzero_scalar(&k2);
        let s3 = nonzero_scalar(&k3);
        let p1 = to_point(&e, &scalar_mul_g(s1));
        let p2 = to_point(&e, &scalar_mul_g(s2));
        let p3 = to_point(&e, &scalar_mul_g(s3));

        let left = Grumpkin::add(&e, &Grumpkin::add(&e, &p1, &p2), &p3);
        let right = Grumpkin::add(&e, &p1, &Grumpkin::add(&e, &p2, &p3));
        prop_assert_eq!(left.to_array(), right.to_array());
    })
}

#[test]
fn prop_identity_law() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    let o = Grumpkin::identity(&e);
    proptest!(|(k: [u8; 32])| {
        let p = to_point(&e, &scalar_mul_g(nonzero_scalar(&k)));
        prop_assert_eq!(Grumpkin::add(&e, &p, &o).to_array(), p.to_array());
        prop_assert_eq!(Grumpkin::add(&e, &o, &p).to_array(), p.to_array());
    })
}

#[test]
fn prop_inversion() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32])| {
        let p = to_point(&e, &scalar_mul_g(nonzero_scalar(&k)));
        let neg_p = Grumpkin::neg(&e, &p);
        prop_assert_eq!(Grumpkin::add(&e, &p, &neg_p).to_array(), [0u8; 64]);
    })
}

#[test]
fn prop_pedersen_homomorphism() {
    // commit(v1, r1) + commit(v2, r2) == commit(v1 + v2, r1 + r2)
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(
        v1: [u8; 32], r1: [u8; 32],
        v2: [u8; 32], r2: [u8; 32],
    )| {
        let v1 = scalar_from_bytes(&v1);
        let r1 = scalar_from_bytes(&r1);
        let v2 = scalar_from_bytes(&v2);
        let r2 = scalar_from_bytes(&r2);

        let c1 = to_point(&e, &pedersen_commit(v1, r1));
        let c2 = to_point(&e, &pedersen_commit(v2, r2));
        let expected = to_point(&e, &pedersen_commit(v1 + v2, r1 + r2));

        prop_assert_eq!(Grumpkin::add(&e, &c1, &c2).to_array(), expected.to_array());
    })
}

#[test]
fn prop_doubling_equals_self_add() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32])| {
        let s = nonzero_scalar(&k);
        let p = to_point(&e, &scalar_mul_g(s));
        let two_p_via_add = Grumpkin::add(&e, &p, &p);
        let two_p_ref = to_point(&e, &scalar_mul_g(s + s));
        prop_assert_eq!(two_p_via_add.to_array(), two_p_ref.to_array());
    })
}

#[test]
fn prop_is_on_curve_holds_for_real_points() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32])| {
        let p = to_point(&e, &scalar_mul_g(nonzero_scalar(&k)));
        prop_assert!(Grumpkin::is_on_curve(&e, &p));
        prop_assert!(Grumpkin::is_not_identity(&p));
    })
}

#[test]
fn prop_is_on_curve_rejects_corrupted_y() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32], byte_idx in 32usize..64, bit in 0u32..8)| {
        let mut p = to_point(&e, &scalar_mul_g(nonzero_scalar(&k))).to_array();
        p[byte_idx] ^= 1u8 << bit;
        let corrupted = BytesN::from_array(&e, &p);
        // Off-curve corruption flips y; for a random base-field element y',
        // (y')² == x³ − 17 holds with probability ≈ 1/r, so this fails with
        // overwhelming probability.
        prop_assert!(!Grumpkin::is_on_curve(&e, &corrupted));
    })
}

/// BN254 scalar field modulus r in big-endian bytes — duplicated here so the
/// test exercises a value independent of the production constant.
const FR_MODULUS_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];

/// Adds the BN254 scalar field modulus `r` to a canonical 32-byte big-endian
/// coordinate, producing a non-canonical encoding that reduces to the same
/// field element under `Bn254Fr::from_bytes`. Returns `None` if the sum
/// overflows 32 bytes (only happens for `coord` very close to `r`, which is
/// extremely unlikely for randomly generated points).
fn add_modulus_be(coord: &[u8; 32]) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    let mut carry: u16 = 0;
    for i in (0..32).rev() {
        let sum = coord[i] as u16 + FR_MODULUS_BE[i] as u16 + carry;
        out[i] = sum as u8;
        carry = sum >> 8;
    }
    if carry != 0 {
        None
    } else {
        Some(out)
    }
}

#[test]
fn prop_is_on_curve_rejects_non_canonical_x() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32])| {
        let canonical = to_point(&e, &scalar_mul_g(nonzero_scalar(&k))).to_array();
        let mut x = [0u8; 32];
        x.copy_from_slice(&canonical[..32]);
        let Some(x_plus_r) = add_modulus_be(&x) else { return Ok(()); };
        let mut malleated = canonical;
        malleated[..32].copy_from_slice(&x_plus_r);
        let mutated = BytesN::from_array(&e, &malleated);
        // Same logical point under mod-r reduction; distinct bytes.
        prop_assert_ne!(malleated, canonical);
        prop_assert!(!Grumpkin::is_on_curve(&e, &mutated));
    })
}

#[test]
fn prop_is_on_curve_rejects_non_canonical_y() {
    let e = Env::default();
    e.cost_estimate().budget().reset_unlimited();

    proptest!(|(k: [u8; 32])| {
        let canonical = to_point(&e, &scalar_mul_g(nonzero_scalar(&k))).to_array();
        let mut y = [0u8; 32];
        y.copy_from_slice(&canonical[32..]);
        let Some(y_plus_r) = add_modulus_be(&y) else { return Ok(()); };
        let mut malleated = canonical;
        malleated[32..].copy_from_slice(&y_plus_r);
        let mutated = BytesN::from_array(&e, &malleated);
        prop_assert_ne!(malleated, canonical);
        prop_assert!(!Grumpkin::is_on_curve(&e, &mutated));
    })
}

#[test]
fn is_on_curve_rejects_x_equal_to_modulus() {
    // x = r is the smallest non-canonical value that reduces to 0; check
    // exactly that boundary.
    let e = Env::default();
    let mut bytes = [0u8; 64];
    bytes[..32].copy_from_slice(&FR_MODULUS_BE);
    let p = BytesN::from_array(&e, &bytes);
    assert!(!Grumpkin::is_on_curve(&e, &p));
}
