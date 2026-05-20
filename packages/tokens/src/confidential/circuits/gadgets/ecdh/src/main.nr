use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{assert_on_curve_non_identity, ecdh, H};

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

#[test]
fn matches_fixture() {
    // Pinned from testdata/ecdh.json: ecdh(r_e, H) where r_e = 0xfeedface.
    main(
        0xfeedface,
        H.x,
        H.y,
        0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f,
    );
}
