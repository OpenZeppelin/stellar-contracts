use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, pvk_from_vk, scalar_mul, vk_from_sk,
};

mod tests;

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
