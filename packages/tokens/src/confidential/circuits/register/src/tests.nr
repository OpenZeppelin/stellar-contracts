use crate::main;
use stellar_confidential_lib::{pvk_from_vk, vk_from_sk};

// Canonical fixture inputs (pinned in lib/testdata/*.json and asserted by
// `fixtures_match_testdata` in lib's tests). Reusing them keeps the SDK
// contract and the per-circuit happy path tied to the same vectors.
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global Y_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;
global PVK_X: Field = 0x2e7421c0e86a4c8eed823edf851c364ecee448bf70eae2a95befe3dd9364cc45;
global PVK_Y: Field = 0x0b060655740aabef84e7305bb886b75da8d258f346381f03925d69645292fe7a;

#[test]
fn matches_fixture() {
    main(SK, Y_X, Y_Y, PVK_X, PVK_Y, ADDR_F);
}

#[test(should_fail)]
fn rejects_tampered_pvk() {
    // PVK.x +1: R3 sees pvk_derived != (pvk_x, pvk_y).
    main(SK, Y_X, Y_Y, PVK_X + 1, PVK_Y, ADDR_F);
}

#[test(should_fail)]
fn rejects_wrong_addr_f() {
    // Y / PVK match (sk, addr_f=0xbeef). A different addr_f recomputes vk under
    // that addr_f; PVK = vk*H then mismatches under R3.
    main(SK, Y_X, Y_Y, PVK_X, PVK_Y, 0xcafe);
}

#[test(should_fail)]
fn rejects_off_curve_y() {
    // (1, 2) is off-curve. R1 fails because scalar_mul(sk, H) yields an
    // on-curve point that cannot equal (1, 2).
    main(SK, 1, 2, PVK_X, PVK_Y, ADDR_F);
}

#[test(should_fail)]
fn rejects_identity_y() {
    // Y = (0, 0) is the identity encoding. To pass R1, sk*H would have to
    // equal (0, 0), which means sk = 0 -- but R4 rules that out before R1
    // can see it. Either way: rejected.
    main(SK, 0, 0, PVK_X, PVK_Y, ADDR_F);
}

#[test(should_fail)]
fn rejects_sk_zero() {
    // sk = 0 is the exact attack R4 closes: without it the prover could
    // publish Y = O and collapse every ECDH involving the spending key.
    // Y / PVK are fixture values that would otherwise be rejected here
    // because they don't derive from sk = 0 -- R4 fires first.
    main(0, Y_X, Y_Y, PVK_X, PVK_Y, ADDR_F);
}

#[test]
fn vk_intermediate_matches_lib() {
    // Sanity: confirm the canonical PVK fixture was derived from the same vk
    // the lib computes. If this diverges, lib/testdata is out of sync with
    // the Register fixture above and one of them must be regenerated.
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk = pvk_from_vk(vk);
    assert(pvk.x == PVK_X);
    assert(pvk.y == PVK_Y);
}
