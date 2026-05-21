use crate::main;
use stellar_confidential_lib::{pvk_from_vk, vk_from_sk};

// Canonical fixture inputs (pinned in lib/testdata/*.json and asserted by
// `fixtures_match_testdata` in lib's tests). Reusing them keeps the SDK
// contract and the per-circuit happy path tied to the same vectors.
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
