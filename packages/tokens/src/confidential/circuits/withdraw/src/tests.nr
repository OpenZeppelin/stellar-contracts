use crate::main;
use stellar_confidential_lib::{
    commit, derive_spend_r, ecdh, encrypt_auditor_sender_balance, encrypt_balance, H, scalar_mul,
    vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SK / ADDR_F / SIGMA / V / R reuse the lib's pinned vectors (lib/testdata)
// so that Y, C_spend, vk, r_new, and b_tilde are derivable from -- and
// pinned by -- the lib's `fixtures_match_testdata` test. A is chosen
// strictly less than V so the happy path exercises a non-trivial
// `v_new = v - a` while still satisfying W4. R_E and K_AUD_S are local
// to this circuit (no auditor-side primitives in lib's fixtures).
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global SIGMA: Field = 0x01;
global V: Field = 1000;
global R: Field = 42;
global A: Field = 300;
global V_NEW: Field = 700;
global R_E: Field = 0xfeedface;

// K_aud_s = `sk_aud * H` for a placeholder auditor secret 0xc0ffee. Any
// non-identity Grumpkin point would do; reusing scalar_mul on a constant
// scalar keeps the value derivable from lib primitives.
global K_AUD_S_SCALAR: Field = 0xc0ffee;

// Y = sk*H (= lib `scalar_mul_Y` fixture).
global Y_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;

// C_spend = commit(1000, 42) (= lib `commit` fixture).
global C_SPEND_X: Field =
    0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05;
global C_SPEND_Y: Field =
    0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e;

// C_spend' and b_tilde are pinned by `withdraw_fixtures_match_lib` below,
// which re-derives them from the lib primitives and asserts the values
// declared here -- if either side drifts the test fails immediately.
global C_SPEND_NEW_X: Field =
    0x22f167015ee0e4b33dcb278cdc824c336c919dc4cd06f391850305d5083336e6;
global C_SPEND_NEW_Y: Field =
    0x0be3d0fda4f8f70b04e2d609db4e29b30345da9288bf208c7560572b2bef172d;
global B_TILDE: Field =
    0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c5670;

// Auditor-block fixtures. Pinned by `withdraw_auditor_fixtures_match_lib`.
// R_E_X (= (R_E * H).x) matches the lib's `ecdh` fixture (`ecdh(r_e, H)` for
// the same r_e), pinning the cross-circuit relationship between R_e and the
// lib's scalar-mul output.
global K_AUD_S_X: Field =
    0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_S_Y: Field =
    0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global B_TILDE_AUD_S: Field =
    0x0a607feb31e56e89ed94a3e1dc3811d3d23aaf10b2f27874ce670878b9f6f071;

#[test]
fn print_fixtures() {
    // One-shot harness: prints C_spend', b_tilde, K_aud_s, R_e, b_tilde_aud_s
    // for the chosen fixture tuple. Run with
    // `nargo test --package circuit_withdraw print_fixtures --show-output`
    // and paste the values into the globals above.
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let b_tilde_aud_s = encrypt_auditor_sender_balance(V_NEW, s_a_s_x, SIGMA);
    let cx = c_new.x;
    let cy = c_new.y;
    let kx = k_aud_s.x;
    let ky = k_aud_s.y;
    let rx = r_e_pt.x;
    let ry = r_e_pt.y;
    println(f"C_SPEND_NEW_X = {cx}");
    println(f"C_SPEND_NEW_Y = {cy}");
    println(f"B_TILDE = {b_tilde}");
    println(f"K_AUD_S_X = {kx}");
    println(f"K_AUD_S_Y = {ky}");
    println(f"R_E_X = {rx}");
    println(f"R_E_Y = {ry}");
    println(f"B_TILDE_AUD_S = {b_tilde_aud_s}");
}

#[test]
fn withdraw_fixtures_match_lib() {
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let y = scalar_mul(SK, H);
    let c_spend = commit(V, R);

    assert(c_new.x == C_SPEND_NEW_X);
    assert(c_new.y == C_SPEND_NEW_Y);
    assert(b_tilde == B_TILDE);
    assert(y.x == Y_X);
    assert(y.y == Y_Y);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
}

#[test]
fn withdraw_auditor_fixtures_match_lib() {
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let b_tilde_aud_s = encrypt_auditor_sender_balance(V_NEW, s_a_s_x, SIGMA);
    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(b_tilde_aud_s == B_TILDE_AUD_S);
}

#[test]
fn matches_fixture() {
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test]
fn full_withdrawal() {
    // Boundary: a = v, so v_new = 0. C_spend' commits to (0, r_new) which is
    // a valid commitment to zero. b_tilde and b_tilde_aud_s both bind to 0
    // (plus mask) so the auditor sees the post-withdrawal balance reach zero.
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(0, r_new);
    let b_tilde = encrypt_balance(0, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let b_tilde_aud_s = encrypt_auditor_sender_balance(0, s_a_s_x, SIGMA);
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        V,
        c_new.x,
        c_new.y,
        SIGMA,
        b_tilde,
        R_E_X,
        R_E_Y,
        b_tilde_aud_s,
    );
}

#[test(should_fail)]
fn rejects_under_funded_withdrawal() {
    // a > v: v - a underflows in the field to a value >= 2^128. W4 fires.
    let a_too_large: Field = V + 1;
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_invalid = commit(V - a_too_large, r_new);
    let b_tilde_invalid = encrypt_balance(V - a_too_large, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let b_tilde_aud_s_invalid = encrypt_auditor_sender_balance(V - a_too_large, s_a_s_x, SIGMA);
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        a_too_large,
        c_new_invalid.x,
        c_new_invalid.y,
        SIGMA,
        b_tilde_invalid,
        R_E_X,
        R_E_Y,
        b_tilde_aud_s_invalid,
    );
}

#[test(should_fail)]
fn rejects_v_out_of_range() {
    // 2^127 is exactly the boundary W4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). C_spend must be recomputed against v_huge so W3
    // passes and W4 is the load-bearing assertion; downstream constraints
    // (W6/W7/W_a4) are never reached.
    let v_huge: Field = 0x80000000000000000000000000000000;
    let c_spend = commit(v_huge, R);
    main(
        SK,
        v_huge,
        R,
        R_E,
        c_spend.x,
        c_spend.y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_a_out_of_range() {
    // a = 2^127: W4's range check on a fires at the exact SEP-41 boundary.
    let a_huge: Field = 0x80000000000000000000000000000000;
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        a_huge,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_sk() {
    main(
        SK + 1,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_balance_opening() {
    main(
        SK,
        V + 1,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_addr_f() {
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        0xcafe,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde() {
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE + 1,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_spend_new() {
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X + 1,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // W8 fires: r_e = 0 would force R_e = O (identity) and collapse
    // S_a,s to O, making m_b a knowable constant function of sigma.
    main(
        SK,
        V,
        R,
        0,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: W_a1 fails because scalar_mul(r_e, H)
    // no longer equals the claimed (R_e_x, R_e_y).
    main(
        SK,
        V,
        R,
        R_E + 1,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    // (1, 2) is off-curve: 4 != 1 - 17. The on-curve check in main()
    // fires before W_a2 ever consumes K_aud_s.
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        1,
        2,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    // K_aud_s = identity collapses ECDH (S = r_e * O = O), so the on-curve
    // check explicitly rejects the identity for keys.
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        0,
        0,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde_aud_s() {
    // b_tilde_aud_s mutated by +1: W_a3+W_a4 fail because
    // encrypt_auditor_sender_balance derives a specific scalar that no
    // longer matches the prover's claim. Closes the
    // tampered-ciphertext criterion in the issue.
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S + 1,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_s() {
    // K_aud_s replaced by a *valid* but different on-curve key (Y_X, Y_Y is
    // sk*H from the fixtures, definitely on-curve and non-identity). The
    // shared secret changes, m_b changes, W_a4 fails.
    main(
        SK,
        V,
        R,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        Y_X,
        Y_Y,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
        R_E_X,
        R_E_Y,
        B_TILDE_AUD_S,
    );
}
