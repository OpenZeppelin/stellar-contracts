use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{ecdh, H};

// Minimal circuit wrapping `ecdh(scalar, point) -> shared_secret_x`.
//
// Note: `(px, py)` is reconstructed into an `EmbeddedCurvePoint` without an
// on-curve check, which a security review might flag. This is by design --
// per design doc Section 7.1, point-on-curve and non-identity validation for
// every point that enters the system from calldata (commitments, ephemeral
// keys, PVKs, auditor keys) is enforced by the *wrapper contract*, not by
// the circuits consuming those points as public inputs. Adding the check
// in-circuit here would inflate this gadget's constraint count so it no
// longer reflects the production circuits' real cost (they inherit
// pre-validated points and skip the check too).
fn main(scalar: Field, px: pub Field, py: pub Field, s: pub Field) {
    let p = EmbeddedCurvePoint { x: px, y: py, is_infinite: false };
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
