use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, pvk_from_vk, scalar_mul, vk_from_sk,
};

// Register circuit -- design doc Section 7.2.
//
// Constraints:
//   R1: Y   = sk * H                              (spending key well-formed)
//   R2: vk  = Poseidon2(delta_vk, sk, wrap)       (wrapper-bound viewing key)
//   R3: PVK = vk * H                              (public viewing key matches vk)
//
// Public inputs (ordering frozen in circuits/PUBLIC_INPUTS.md):
//   0: Y.x   1: Y.y   2: PVK.x   3: PVK.y   4: wrap
// Private witness: sk.
fn main(
    sk: Field,
    y_x: pub Field,
    y_y: pub Field,
    pvk_x: pub Field,
    pvk_y: pub Field,
    wrap: pub Field,
) {
    // Section 7.1 point 4: Y and PVK are public-input keys -- both must lie on
    // Grumpkin and reject the identity. The verifier doesn't enforce this; if
    // skipped, a prover can publish the identity for which every ECDH secret
    // collapses to zero.
    let y = EmbeddedCurvePoint { x: y_x, y: y_y, is_infinite: false };
    let pvk = EmbeddedCurvePoint { x: pvk_x, y: pvk_y, is_infinite: false };
    assert_on_curve_non_identity(y);
    assert_on_curve_non_identity(pvk);

    // R1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // R2 + R3 (vk is purely intermediate; never exposed)
    let vk = vk_from_sk(sk, wrap);
    let pvk_derived = pvk_from_vk(vk);
    assert(pvk_derived.x == pvk_x);
    assert(pvk_derived.y == pvk_y);
}

// ################## TESTS ##################

// Canonical fixture inputs (pinned in lib/testdata/*.json and asserted by
// `fixtures_match_testdata` in lib.nr). Reusing them keeps the SDK contract
// and the per-circuit happy path tied to the same vectors.
global SK: Field = 0xdead;
global WRAP: Field = 0xbeef;
global Y_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;
global PVK_X: Field = 0x01de20bfe21ad660e073b14a106feae4afce4e78a020953ca16b767e0615331b;
global PVK_Y: Field = 0x12cf407b75fccb5c7721c5fdadc3d1a22b5440e36b08fe52658ab885529e92fd;

#[test]
fn matches_fixture() {
    main(SK, Y_X, Y_Y, PVK_X, PVK_Y, WRAP);
}

#[test(should_fail)]
fn rejects_tampered_pvk() {
    // Perturb PVK.x by +1. The point almost certainly leaves the curve, which
    // trips assert_on_curve_non_identity first; on the unlikely chance it stays
    // on-curve, R3 sees PVK != vk*H and fails there. Either way: rejected.
    main(SK, Y_X, Y_Y, PVK_X + 1, PVK_Y, WRAP);
}

#[test(should_fail)]
fn rejects_wrong_wrap() {
    // Y and PVK are the values that match (sk, wrap=0xbeef). Passing a
    // different wrap recomputes vk under that wrap; PVK = vk*H then mismatches.
    main(SK, Y_X, Y_Y, PVK_X, PVK_Y, 0xcafe);
}

#[test(should_fail)]
fn rejects_off_curve_y() {
    // (1, 2) fails Grumpkin's y^2 = x^3 - 17: 4 != 1 - 17 = -16.
    main(SK, 1, 2, PVK_X, PVK_Y, WRAP);
}

#[test(should_fail)]
fn rejects_identity_y() {
    // Encoding Y as (0, 0) -- the canonical identity -- with is_infinite=false
    // (which is what the public-input wiring forces) fails the on-curve check:
    // 0 != 0 - 17. Documents that the identity cannot slip through as a key.
    main(SK, 0, 0, PVK_X, PVK_Y, WRAP);
}

#[test]
fn vk_intermediate_matches_lib() {
    // Sanity: confirm the canonical PVK fixture was derived from the same vk
    // the lib computes. If this diverges, lib/testdata is out of sync with
    // the Register fixture above and one of them must be regenerated.
    let vk = vk_from_sk(SK, WRAP);
    let pvk = pvk_from_vk(vk);
    assert(pvk.x == PVK_X);
    assert(pvk.y == PVK_Y);
}
