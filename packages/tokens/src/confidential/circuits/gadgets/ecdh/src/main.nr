use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{assert_on_curve_non_identity, ecdh};

// Minimal circuit wrapping `ecdh(scalar, point) -> shared_secret_x`.
//
// The on-curve / non-identity check is part of the gadget by design:
// production circuits must validate every public-input key (R_e, Y, PVK,
// K_aud) before passing it to a scalar multiplication -- the verifier itself
// does not check curve membership, so a missed validation breaks soundness.
// Including it here means the measured constraint count reflects what a real
// circuit will actually pay.
fn main(scalar: Field, px: pub Field, py: pub Field, s: pub Field) {
    let p = EmbeddedCurvePoint { x: px, y: py, is_infinite: false };
    assert_on_curve_non_identity(p);
    let out = ecdh(scalar, p);
    assert(out == s);
}
