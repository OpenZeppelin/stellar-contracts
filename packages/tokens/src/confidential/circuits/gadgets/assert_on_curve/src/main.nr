use std::embedded_curve_ops::EmbeddedCurvePoint;
use stellar_confidential_lib::{assert_on_curve_non_identity, G};

// Minimal circuit wrapping `assert_on_curve_non_identity(p)`. Measures the
// constraint cost of validating one public-input key (the variant that
// rejects the identity, per design doc Section 7.1 point 4).
fn main(px: pub Field, py: pub Field) {
    let p = EmbeddedCurvePoint { x: px, y: py, is_infinite: false };
    assert_on_curve_non_identity(p);
}

#[test]
fn accepts_G() {
    main(G.x, G.y);
}

#[test(should_fail)]
fn rejects_off_curve() {
    main(1, 2);
}
