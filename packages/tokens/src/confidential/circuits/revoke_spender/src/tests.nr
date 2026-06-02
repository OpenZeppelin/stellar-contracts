use crate::main;
use stellar_confidential_lib::{
    commit, derive_allow_r, derive_spend_r, domain, dvk_from_vk_op, ecdh, encrypt_balance, H,
    scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SK / ADDR_F / SIGMA / SIGMA_A / OP_I / V_S / R_S / V_A / R_E reuse the lib's
// pinned vectors (lib/testdata) so that Y, C_spend, C_a, vk, dvk_i, r_a, r_new,
// and b_tilde are derivable from -- and pinned by -- the lib's
// `fixtures_match_testdata` test. V_S + V_A = 1500 is well below 2^127 so the
// happy path exercises a non-trivial sum while still satisfying V9. R_E and
// K_AUD_S are local to this circuit (no auditor-side primitives in lib's
// fixtures).
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global SIGMA: Field = 0x01;
global SIGMA_A: Field = 0x02;
global OP_I: Field = 0xabcd;
global V_S: Field = 1000;
global R_S: Field = 42;
global V_A: Field = 500;
global V_NEW: Field = 1500;
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

// r_a = derive_allow_r(dvk_i, sigma_a) (= lib fixture).
global R_A: Field = 0x0afda20e062a277afc0c86e31ea22597ee2c094b1b9cd08b8d8a2a212a8af1ee;

// C_a = commit(V_A, R_A). Pinned by `revoke_spender_fixtures_match_lib`.
global C_A_X: Field =
    0x008a13da4a758b5119439c631f9ec3d32936432549b48ec3ccfb9febe3e33cd1;
global C_A_Y: Field =
    0x233c5490e39ce0276b467a503d4216304c901a08ff6af827d12b28d3376f8e06;

// C_spend' and b_tilde are pinned by `revoke_spender_fixtures_match_lib`,
// which re-derives them from the lib primitives and asserts the values
// declared here -- if either side drifts the test fails immediately.
global C_SPEND_NEW_X: Field =
    0x1a591f0cb6df2ec210dd1319f9b6af34bdee16c4325cf74bd29a14d4d45acb28;
global C_SPEND_NEW_Y: Field =
    0x2d3be2c130a15ece090a6c7ee42fa54880cec3861b9ade4691a23e37d1687d5c;
global B_TILDE: Field =
    0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c5990;

// Auditor-block fixtures. Pinned by `revoke_spender_auditor_fixtures_match_lib`.
// R_E_X (= (R_E * H).x) matches the lib's `ecdh` fixture (`ecdh(r_e, H)` for
// the same r_e), pinning the cross-circuit relationship between R_e and the
// lib's scalar-mul output.
global K_AUD_S_X: Field =
    0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_S_Y: Field =
    0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global V_TILDE_AUD_S: Field =
    0x0a607feb31e56e89ed94a3e1dc3811d3d23aaf10b2f27874ce670878b9f6efa9;
global B_TILDE_AUD_S: Field =
    0x0b586defafdf4583d0f338855fdf8391102adaac4634a3b80594aea6f788e423;

#[test]
fn print_fixtures() {
    // One-shot harness: prints C_a, C_spend', b_tilde, K_aud_s, R_e,
    // v_tilde_aud_s, b_tilde_aud_s for the chosen fixture tuple. Run with
    // `nargo test --package circuit_revoke_spender print_fixtures --show-output`
    // and paste the values into the globals above.
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk_i = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk_i, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = V_A + m[0];
    let b_tilde_aud_s = V_NEW + m[1];
    let ax = c_a.x;
    let ay = c_a.y;
    let cx = c_new.x;
    let cy = c_new.y;
    let kx = k_aud_s.x;
    let ky = k_aud_s.y;
    let rx = r_e_pt.x;
    let ry = r_e_pt.y;
    println(f"R_A = {r_a}");
    println(f"C_A_X = {ax}");
    println(f"C_A_Y = {ay}");
    println(f"C_SPEND_NEW_X = {cx}");
    println(f"C_SPEND_NEW_Y = {cy}");
    println(f"B_TILDE = {b_tilde}");
    println(f"K_AUD_S_X = {kx}");
    println(f"K_AUD_S_Y = {ky}");
    println(f"R_E_X = {rx}");
    println(f"R_E_Y = {ry}");
    println(f"V_TILDE_AUD_S = {v_tilde_aud_s}");
    println(f"B_TILDE_AUD_S = {b_tilde_aud_s}");
}

#[test]
fn revoke_spender_fixtures_match_lib() {
    // Pins the main-block fixtures (C_a, C_spend', b_tilde, Y, C_spend, R_A)
    // to the lib primitives. If lib derivations or fixture values drift, this
    // test catches it before `matches_fixture` reports a less informative
    // failure deep in `main`.
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk_i = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk_i, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let y = scalar_mul(SK, H);
    let c_spend = commit(V_S, R_S);

    assert(r_a == R_A);
    assert(c_a.x == C_A_X);
    assert(c_a.y == C_A_Y);
    assert(c_new.x == C_SPEND_NEW_X);
    assert(c_new.y == C_SPEND_NEW_Y);
    assert(b_tilde == B_TILDE);
    assert(y.x == Y_X);
    assert(y.y == Y_Y);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
}

#[test]
fn revoke_spender_auditor_fixtures_match_lib() {
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = V_A + m[0];
    let b_tilde_aud_s = V_NEW + m[1];
    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(v_tilde_aud_s == V_TILDE_AUD_S);
    assert(b_tilde_aud_s == B_TILDE_AUD_S);
}

#[test]
fn matches_fixture() {
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test]
fn alternate_opening() {
    // Second happy path with a different (V_S, R_S, V_A, SIGMA_A) tuple to
    // exercise the full constraint set against an independent witness. All
    // intermediate values are re-derived from lib primitives, so this test
    // is also a regression guard for any lib drift specific to the
    // alternate-opening path.
    let v_s_alt: Field = 7777;
    let r_s_alt: Field = 0xabc123;
    let v_a_alt: Field = 12345;
    let v_new_alt: Field = v_s_alt + v_a_alt;
    let sigma_a_alt: Field = 0x09;

    let c_spend_alt = commit(v_s_alt, r_s_alt);
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk_i = dvk_from_vk_op(vk, OP_I);
    let r_a_alt = derive_allow_r(dvk_i, sigma_a_alt);
    let c_a_alt = commit(v_a_alt, r_a_alt);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_alt, r_new);
    let b_tilde = encrypt_balance(v_new_alt, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = v_a_alt + m[0];
    let b_tilde_aud_s = v_new_alt + m[1];

    main(
        SK,
        v_a_alt,
        r_a_alt,
        v_s_alt,
        r_s_alt,
        R_E,
        c_spend_alt.x,
        c_spend_alt.y,
        c_a_alt.x,
        c_a_alt.y,
        sigma_a_alt,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        b_tilde,
        SIGMA,
        R_E_X,
        R_E_Y,
        v_tilde_aud_s,
        b_tilde_aud_s,
    );
}

#[test(should_fail)]
fn rejects_wrong_sk() {
    // V1 fires: scalar_mul(sk+1, H) != Y.
    main(
        SK + 1,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_addr_f() {
    // V2 binds vk to addr_f. A bogus addr_f derives a different vk, which
    // propagates to V6 (r'), V7 (C_spend'), and V8 (b_tilde). The earliest
    // assertion that observes the mismatch (C_spend' mismatch) fires.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        0xcafe,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_dvk_op_i() {
    // V3 binds dvk_i to op_i. A bogus op_i derives a different dvk_i, which
    // makes r_a_derived != R_A in V4 and the assertion fires.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I + 1,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_bad_allowance_opening_value() {
    // V4 fires: v_a + 1 is not a valid opening of the on-chain C_a under the
    // bound r_a (which is itself bound by derive_allow_r to dvk_i, sigma_a).
    main(
        SK,
        V_A + 1,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_bad_allowance_opening_randomness() {
    // V4 fires first via r_a_derived != r_a: the prover-supplied r_a no
    // longer matches Poseidon2(delta_allow_r, dvk_i, sigma_a).
    main(
        SK,
        V_A,
        R_A + 1,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_sigma_a() {
    // V4 fires: a different sigma_a re-derives r_a_derived which no longer
    // matches the supplied R_A.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A + 1,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_bad_spend_opening() {
    // V5 fires: V_S + 1 with the original R_S no longer recommits to
    // (C_SPEND_X, C_SPEND_Y).
    main(
        SK,
        V_A,
        R_A,
        V_S + 1,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_v_s_out_of_range() {
    // V9 fires: 2^127 is the smallest value outside [0, 2^127) (SEP-41 i128
    // non-negative range, Section 2.6 / 3.4). C_spend must be recomputed
    // against v_s_huge so V5 passes and V9's first range check is the
    // load-bearing assertion.
    let v_s_huge: Field = 0x80000000000000000000000000000000;
    let c_spend = commit(v_s_huge, R_S);
    main(
        SK,
        V_A,
        R_A,
        v_s_huge,
        R_S,
        R_E,
        c_spend.x,
        c_spend.y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_v_a_out_of_range() {
    // V9 fires on the v_a range check. C_a is recomputed against v_a_huge so
    // V4 passes (r_a still binds via derive_allow_r) and V9 is the
    // load-bearing assertion.
    let v_a_huge: Field = 0x80000000000000000000000000000000;
    let c_a = commit(v_a_huge, R_A);
    main(
        SK,
        v_a_huge,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        c_a.x,
        c_a.y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_v_sum_overflow() {
    // SOUNDNESS-CRITICAL: the only line of defense against a addr_f that lets
    // the owner mint tokens out of nothing on the reclaim path.
    //
    // Witnesses chosen so that the individual range checks on v_s and v_a
    // each pass (both are < 2^127), but the sum v_s + v_a = 2^127 sits at
    // the exact SEP-41 i128 boundary:
    //
    //   v_s = 2^127 - 1   (largest value passing the 127-bit check)
    //   v_a = 1
    //   v_s + v_a = 2^127 (smallest value failing the 127-bit check)
    //
    // All downstream public inputs (C_spend, C_spend', C_a, b_tilde, the
    // auditor ciphertexts) are recomputed *consistently* with the prover's
    // claimed (v_s, v_a, v_new = v_s + v_a) values, so V1-V8 and V_a1-V_a5
    // all succeed individually. The failure is unambiguously the
    // `(v_s + v_a).assert_max_bit_size::<127>()` line in V9. If this guard
    // ever regressed, the proof would verify and the owner would mint
    // 2^127 - existing_spend tokens on revoke. Treat as load-bearing.
    let v_s_huge: Field = 0x7fffffffffffffffffffffffffffffff; // 2^127 - 1
    let v_a_one: Field = 1;
    let v_new_overflow: Field = v_s_huge + v_a_one; // 2^127
    let c_spend = commit(v_s_huge, R_S);
    let c_a = commit(v_a_one, R_A);
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_overflow, r_new);
    let b_tilde = encrypt_balance(v_new_overflow, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = v_a_one + m[0];
    let b_tilde_aud_s = v_new_overflow + m[1];
    main(
        SK,
        v_a_one,
        R_A,
        v_s_huge,
        R_S,
        R_E,
        c_spend.x,
        c_spend.y,
        c_a.x,
        c_a.y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        b_tilde,
        SIGMA,
        R_E_X,
        R_E_Y,
        v_tilde_aud_s,
        b_tilde_aud_s,
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // V10 fires: r_e = 0 would force R_e = O (identity) and collapse
    // S_{a,s} to O, making (m_v, m_b) knowable constant functions of sigma.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        0,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // V_a1 fires: scalar_mul(r_e + 1, H) no longer equals the claimed
    // (R_e_x, R_e_y).
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E + 1,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    // (1, 2) is off-curve: 4 != 1 - 17. The on-curve check in main()
    // fires before V_a2 ever consumes K_aud_s.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        1,
        2,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    // K_aud_s = identity collapses ECDH (S = r_e * O = O), so the on-curve
    // check explicitly rejects the identity for keys.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        0,
        0,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde() {
    // V8 fires: encrypt_balance(v_new, vk, sigma) no longer matches the
    // tampered b_tilde.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE + 1,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_spend_new() {
    // V7 fires: (v_s + v_a) * G + r' * H no longer equals the tampered
    // (C_spend'.x + 1, C_spend'.y).
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X + 1,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_s() {
    // V_a4 fires: v_a + m_v no longer equals the tampered v_tilde_aud_s.
    // Closes the auditor-amount tampered-ciphertext criterion.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S + 1,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde_aud_s() {
    // V_a5 fires: (v_s + v_a) + m_b no longer equals the tampered
    // b_tilde_aud_s. Closes the auditor-balance tampered-ciphertext
    // criterion.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S + 1,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_s() {
    // K_aud_s replaced by a *valid* but different on-curve key (Y_X, Y_Y is
    // sk*H from the fixtures, definitely on-curve and non-identity). The
    // shared secret changes, (m_v, m_b) change, V_a4 fails first.
    main(
        SK,
        V_A,
        R_A,
        V_S,
        R_S,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        Y_X,
        Y_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        B_TILDE,
        SIGMA,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}
