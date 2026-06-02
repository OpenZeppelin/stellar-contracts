use crate::main;
use stellar_confidential_lib::{
    commit, derive_allow_r, derive_spend_r, domain, dvk_from_vk_op, ecdh, encrypt_allowance,
    encrypt_balance, encrypt_esc_dvk, H, scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SK / ADDR_F / SIGMA / V / R / R_E reuse the lib's pinned vectors (lib/testdata)
// so that Y, C_spend, vk, r_new, b_tilde, R_e, and the owner-channel masks
// are all derivable from -- and pinned by -- the lib's `fixtures_match_testdata`
// test. V_A = 300 keeps the happy path on a non-trivial v - v_a while still
// satisfying S4 (matches the withdraw fixture's A so the owner-block balance
// math is identical across the two owner-initiated circuits). SIGMA_A is a
// distinct salt so the allowance channel is independent of the owner channel.
// SK_OP / K_AUD_S_SCALAR are placeholder secret scalars used to materialize
// on-curve Y_op / K_aud_s; any non-zero scalar would do, these are pinned by
// `set_spender_fixtures_match_lib`. K_AUD_S_SCALAR matches the withdraw
// fixture's auditor scalar so the two circuits share the same owner-auditor
// key in cross-circuit tooling. OP_I is a placeholder address-as-field; the
// contract computes it off-circuit via `address_to_field` (Section 2.7).
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global SIGMA: Field = 0x01;
global SIGMA_A: Field = 0x02;
global V: Field = 1000;
global R: Field = 42;
global V_A: Field = 300;
global V_NEW: Field = 700;
global R_E: Field = 0xfeedface;
global OP_I: Field = 0xfee1;

// Placeholder spender secret scalar; Y_op = SK_OP * H.
global SK_OP: Field = 0xbabe;
// Placeholder owner-auditor secret scalar; K_aud_s = scalar * H. Matches the
// withdraw fixture so cross-circuit tooling sees the same K_aud_s.
global K_AUD_S_SCALAR: Field = 0xc0ffee;

// Y = sk*H (= lib `scalar_mul_Y` fixture).
global Y_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;

// C_spend = commit(1000, 42) (= lib `commit` fixture).
global C_SPEND_X: Field =
    0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05;
global C_SPEND_Y: Field =
    0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e;

// K_aud_s = K_AUD_S_SCALAR * H. R_e = R_E * H. Both pinned by the withdraw
// fixture (`withdraw_auditor_fixtures_match_lib`) since they reuse the same
// scalars.
global K_AUD_S_X: Field =
    0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_S_Y: Field =
    0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;

// C_spend' and b_tilde reuse the withdraw fixture: V_A = A = 300 here, and
// the owner-block randomness (vk, sigma) is identical, so both spendable-
// balance ciphertexts collapse to the withdraw values. Pinned by
// `withdraw_fixtures_match_lib` and re-asserted by
// `set_spender_fixtures_match_lib`.
global C_SPEND_NEW_X: Field =
    0x22f167015ee0e4b33dcb278cdc824c336c919dc4cd06f391850305d5083336e6;
global C_SPEND_NEW_Y: Field =
    0x0be3d0fda4f8f70b04e2d609db4e29b30345da9288bf208c7560572b2bef172d;
global B_TILDE: Field =
    0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c5670;

// New values pinned by `set_spender_fixtures_match_lib` and
// `set_spender_auditor_fixtures_match_lib` (re-derived from the lib
// primitives -- if either side drifts the test fails immediately). Emitted by
// `print_fixtures`.
global Y_OP_X: Field = 0x08a500f1d05ebcba6cdbe9c246475e406f28ee2285fdfa4ed9781013ebb1571e;
global Y_OP_Y: Field = 0x2264cba150d089d7678ee412307b118826efb9e4c0f1b4ae10a851d63e83c632;
global C_A_X: Field = 0x1c673e9370f2dcf333e4c63d6faf3957e5a667017406540713b248583c5ae7d7;
global C_A_Y: Field = 0x1f9f86c84bed1f091c282300a063e4ff38082a63aeec17aef054d5ece0c3ebcc;
global A_TILDE: Field =
    0x0c4647bbcd2819afdcd6facf2037861da37e68008f3a40ec2f8db6487e1355c1;
global ESCROWED_DVK_CIPHER: Field =
    0x0d9a3beebbda646b71859a11420a74f8b3fe645d9f235f862733554e1d928405;
global V_TILDE_AUD_S: Field =
    0x0a607feb31e56e89ed94a3e1dc3811d3d23aaf10b2f27874ce670878b9f6eee1;
global B_TILDE_AUD_S: Field =
    0x0b586defafdf4583d0f338855fdf8391102adaac4634a3b80594aea6f788e103;

#[test]
fn print_fixtures() {
    // One-shot harness: prints every derived public input for the chosen
    // fixture tuple. Run with
    // `nargo test --package circuit_set_spender print_fixtures --show-output`
    // and paste the values into the globals above.
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let a_tilde = encrypt_allowance(V_A, dvk, SIGMA_A);

    let r_new = derive_spend_r(vk, SIGMA);
    let c_spend_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);

    let y_op = scalar_mul(SK_OP, H);
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher = encrypt_esc_dvk(dvk, s_esc, OP_I);

    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = V_A + m_s[0];
    let b_tilde_aud_s = V_NEW + m_s[1];

    let yopx = y_op.x;
    let yopy = y_op.y;
    let cax = c_a.x;
    let cay = c_a.y;
    let csnx = c_spend_new.x;
    let csny = c_spend_new.y;
    let kx = k_aud_s.x;
    let ky = k_aud_s.y;
    let rex = r_e_pt.x;
    let rey = r_e_pt.y;
    println(f"Y_OP_X = {yopx}");
    println(f"Y_OP_Y = {yopy}");
    println(f"C_A_X = {cax}");
    println(f"C_A_Y = {cay}");
    println(f"A_TILDE = {a_tilde}");
    println(f"C_SPEND_NEW_X = {csnx}");
    println(f"C_SPEND_NEW_Y = {csny}");
    println(f"B_TILDE = {b_tilde}");
    println(f"ESCROWED_DVK_CIPHER = {escrowed_dvk_cipher}");
    println(f"K_AUD_S_X = {kx}");
    println(f"K_AUD_S_Y = {ky}");
    println(f"R_E_X = {rex}");
    println(f"R_E_Y = {rey}");
    println(f"V_TILDE_AUD_S = {v_tilde_aud_s}");
    println(f"B_TILDE_AUD_S = {b_tilde_aud_s}");
}

#[test]
fn set_spender_fixtures_match_lib() {
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let a_tilde = encrypt_allowance(V_A, dvk, SIGMA_A);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let y = scalar_mul(SK, H);
    let c_spend = commit(V, R);
    let y_op = scalar_mul(SK_OP, H);

    assert(y.x == Y_X);
    assert(y.y == Y_Y);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
    assert(y_op.x == Y_OP_X);
    assert(y_op.y == Y_OP_Y);
    assert(c_a.x == C_A_X);
    assert(c_a.y == C_A_Y);
    assert(a_tilde == A_TILDE);
    assert(c_new.x == C_SPEND_NEW_X);
    assert(c_new.y == C_SPEND_NEW_Y);
    assert(b_tilde == B_TILDE);
}

#[test]
fn set_spender_auditor_fixtures_match_lib() {
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let y_op = scalar_mul(SK_OP, H);
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher = encrypt_esc_dvk(dvk, s_esc, OP_I);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);

    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(escrowed_dvk_cipher == ESCROWED_DVK_CIPHER);
    assert(V_A + m_s[0] == V_TILDE_AUD_S);
    assert(V_NEW + m_s[1] == B_TILDE_AUD_S);
}

// `run_main` accepts all 24 public inputs as parameters -- nothing baked in.
// Tests that target a specific constraint construct the public tuple so only
// that constraint can fail: tamper the witness or public input under test
// AND recompute every downstream public that depends on it, so no cascaded
// mismatch coincidentally lights up the same `should_fail`.
fn run_main(
    sk_in: Field,
    v_in: Field,
    r_in: Field,
    v_a_in: Field,
    r_e_in: Field,
    c_spend_x_in: Field,
    c_spend_y_in: Field,
    y_x_in: Field,
    y_y_in: Field,
    y_op_x_in: Field,
    y_op_y_in: Field,
    op_i_in: Field,
    addr_f_in: Field,
    k_aud_s_x_in: Field,
    k_aud_s_y_in: Field,
    c_spend_new_x_in: Field,
    c_spend_new_y_in: Field,
    c_a_x_in: Field,
    c_a_y_in: Field,
    escrowed_dvk_r_x_in: Field,
    escrowed_dvk_cipher_in: Field,
    b_tilde_in: Field,
    a_tilde_in: Field,
    sigma_in: Field,
    sigma_a_in: Field,
    r_e_x_in: Field,
    r_e_y_in: Field,
    v_tilde_aud_s_in: Field,
    b_tilde_aud_s_in: Field,
) {
    main(
        sk_in,
        v_in,
        r_in,
        v_a_in,
        r_e_in,
        c_spend_x_in,
        c_spend_y_in,
        y_x_in,
        y_y_in,
        y_op_x_in,
        y_op_y_in,
        op_i_in,
        addr_f_in,
        k_aud_s_x_in,
        k_aud_s_y_in,
        c_spend_new_x_in,
        c_spend_new_y_in,
        c_a_x_in,
        c_a_y_in,
        escrowed_dvk_r_x_in,
        escrowed_dvk_cipher_in,
        b_tilde_in,
        a_tilde_in,
        sigma_in,
        sigma_a_in,
        r_e_x_in,
        r_e_y_in,
        v_tilde_aud_s_in,
        b_tilde_aud_s_in,
    );
}

fn run_fixture() {
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test]
fn matches_fixture() {
    run_fixture();
}

#[test]
fn full_allowance_escrow() {
    // Boundary: v_a = v, so v_new = 0. C_spend' commits to (0, r_new) which
    // is a valid commitment to zero. b_tilde and b_tilde_aud_s both bind to 0
    // (plus mask) so the owner's auditor sees the post-escrow spendable
    // balance reach zero. C_a binds to the full v.
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk, SIGMA_A);
    let c_a = commit(V, r_a);
    let a_tilde = encrypt_allowance(V, dvk, SIGMA_A);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(0, r_new);
    let b_tilde = encrypt_balance(0, vk, SIGMA);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher = encrypt_esc_dvk(dvk, s_esc, OP_I);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        V,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        c_a.x,
        c_a.y,
        R_E_X,
        escrowed_dvk_cipher,
        b_tilde,
        a_tilde,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V + m_s[0],
        0 + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_under_funded_escrow() {
    // v_a > v: v - v_a underflows in the field to a value >= 2^128. S4 fires.
    // Every downstream public is recomputed against v_a_too_large so the
    // single firing constraint is S4's v_new range check.
    let v_a_too_large: Field = V + 1;
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk, SIGMA_A);
    let c_a_invalid = commit(v_a_too_large, r_a);
    let a_tilde_invalid = encrypt_allowance(v_a_too_large, dvk, SIGMA_A);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_invalid = commit(V - v_a_too_large, r_new);
    let b_tilde_invalid = encrypt_balance(V - v_a_too_large, vk, SIGMA);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher = encrypt_esc_dvk(dvk, s_esc, OP_I);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        v_a_too_large,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_invalid.x,
        c_new_invalid.y,
        c_a_invalid.x,
        c_a_invalid.y,
        R_E_X,
        escrowed_dvk_cipher,
        b_tilde_invalid,
        a_tilde_invalid,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        v_a_too_large + m_s[0],
        (V - v_a_too_large) + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_out_of_range() {
    // 2^127 is exactly the boundary S4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). With v_huge - V_A = 2^127 - 300 still in [0, 2^127),
    // the v_new range check passes; only the `v.assert_max_bit_size<127>()`
    // sub-check of S4 fires. C_spend, c_spend_new, b_tilde, and b_tilde_aud_s
    // are recomputed against v_huge / v_new_huge so S3 / S10 / S11 / S_a5
    // cannot coincidentally fire.
    let v_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = v_huge - V_A;
    let vk = vk_from_sk(SK, ADDR_F);
    let c_spend = commit(v_huge, R);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_huge, r_new);
    let b_tilde = encrypt_balance(v_new_huge, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        v_huge,
        R,
        V_A,
        R_E,
        c_spend.x,
        c_spend.y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        b_tilde,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        v_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_a_out_of_range() {
    // v_a = 2^127: S4's range check on v_a fires at the exact SEP-41
    // boundary. v_new = V - 2^127 field-underflows so the v_new range check
    // would also fire, but both belong to S4. Every downstream public that
    // depends on v_a (C_a, a_tilde, v_tilde_aud_s) and on v_new (c_spend_new,
    // b_tilde, b_tilde_aud_s) is recomputed so the only constraints that fail
    // are S4's two range sub-checks.
    let v_a_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = V - v_a_huge;
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let r_a = derive_allow_r(dvk, SIGMA_A);
    let c_a = commit(v_a_huge, r_a);
    let a_tilde = encrypt_allowance(v_a_huge, dvk, SIGMA_A);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_huge, r_new);
    let b_tilde = encrypt_balance(v_new_huge, vk, SIGMA);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher = encrypt_esc_dvk(dvk, s_esc, OP_I);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        v_a_huge,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        c_a.x,
        c_a.y,
        R_E_X,
        escrowed_dvk_cipher,
        b_tilde,
        a_tilde,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        v_a_huge + m_s[0],
        v_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_sk() {
    // sk + 1 != sk: S1 fails because scalar_mul(sk+1, H) no longer matches Y.
    // vk is rebound to (sk+1, addr_f) and every vk-dependent public
    // (c_spend_new, b_tilde, c_a, a_tilde, escrowed_dvk_cipher) is recomputed
    // under the new vk so S10 / S11 / S7 / S8 / S12(c) cannot fire alongside
    // S1. The auditor block does not depend on vk so its canonical values
    // still satisfy S_a4 / S_a5.
    let vk_bad = vk_from_sk(SK + 1, ADDR_F);
    let dvk_bad = dvk_from_vk_op(vk_bad, OP_I);
    let r_a_bad = derive_allow_r(dvk_bad, SIGMA_A);
    let c_a_bad = commit(V_A, r_a_bad);
    let a_tilde_bad = encrypt_allowance(V_A, dvk_bad, SIGMA_A);
    let r_new_bad = derive_spend_r(vk_bad, SIGMA);
    let c_new_bad = commit(V_NEW, r_new_bad);
    let b_tilde_bad = encrypt_balance(V_NEW, vk_bad, SIGMA);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let s_esc = ecdh(R_E, y_op);
    let escrowed_dvk_cipher_bad = encrypt_esc_dvk(dvk_bad, s_esc, OP_I);
    run_main(
        SK + 1,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        c_a_bad.x,
        c_a_bad.y,
        R_E_X,
        escrowed_dvk_cipher_bad,
        b_tilde_bad,
        a_tilde_bad,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_balance_opening() {
    // S3 fires: commit(V+1, R) != C_spend. v_new = (V+1) - V_A cascades into
    // S10 / S11 / S_a5; v_a is unchanged so S7 / S8 / S_a4 stay canonical.
    // Recompute c_spend_new, b_tilde, and b_tilde_aud_s against v_new = 701
    // so only S3 fires.
    let v_bad: Field = V + 1;
    let v_new_bad: Field = v_bad - V_A;
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_bad = commit(v_new_bad, r_new);
    let b_tilde_bad = encrypt_balance(v_new_bad, vk, SIGMA);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        v_bad,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        b_tilde_bad,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        v_new_bad + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_addr_f() {
    // The circuit binds vk to addr_f via S2 but doesn't assert addr_f directly;
    // the binding is visible only through vk-derived publics (S10, S11, S7,
    // S8, S12(c)). With addr_f_bad, recompute c_spend_new and c_a from the new
    // vk_bad so S10 / S7 pass -- leaving b_tilde canonical, where the
    // addr_f-binding catches the mismatch via S11. (Recomputing everything,
    // as in `rejects_wrong_sk`, would make the proof verify: S1 is the
    // load-bearing addr_f-independent check there, and no such constraint
    // exists here.)
    let addr_f_bad: Field = 0xcafe;
    let vk_bad = vk_from_sk(SK, addr_f_bad);
    let dvk_bad = dvk_from_vk_op(vk_bad, OP_I);
    let r_a_bad = derive_allow_r(dvk_bad, SIGMA_A);
    let c_a_bad = commit(V_A, r_a_bad);
    let r_new_bad = derive_spend_r(vk_bad, SIGMA);
    let c_new_bad = commit(V_NEW, r_new_bad);
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        addr_f_bad,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        c_a_bad.x,
        c_a_bad.y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_op_i() {
    // op_i changes the derived dvk (S5), which cascades into r_a (S6), C_a
    // (S7), a_tilde (S8), and the escrow cipher (S12(c)). Keeping the
    // canonical C_A / A_TILDE / ESCROWED_DVK_CIPHER public inputs forces all
    // three of S7 / S8 / S12(c) to mismatch -- any one is sufficient to
    // catch the wrong op_i.
    let op_i_bad: Field = OP_I + 1;
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        op_i_bad,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_a() {
    // S7 fires: C_a + 1 no longer matches commit(v_a, r_a).
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X + 1,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_a_tilde() {
    // S8 fires: a_tilde + 1 no longer matches
    // v_a + Poseidon2(ENC_ALLOW, dvk, sigma_a).
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE + 1,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_spend_new() {
    // S10 fires.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X + 1,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde() {
    // S11 fires.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE + 1,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_escrowed_dvk_r_x() {
    // S12(a) fires: escrowed_dvk_r_x must equal R_e.x. Tamper the R_x limb
    // by +1 while keeping the canonical R_e (r_e_x / r_e_y from S_a1
    // unchanged) so the auditor block proceeds normally; only S12(a) catches
    // the mismatch.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X + 1,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_escrowed_dvk_cipher() {
    // S12(c) fires: cipher + 1 no longer matches dvk + Poseidon2(ESC_DVK,
    // s_esc, op_i).
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER + 1,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_y_op() {
    // Y_op replaced by a different on-curve key (Y_X, Y_Y -- the owner's
    // own Y, which is valid on-curve). s_esc = r_e * Y_op changes, so
    // escrowed_dvk_cipher under the canonical dvk no longer matches and
    // S12(c) fails. (Y_op is path (2) of Section 10.8: trusted on read from
    // the contract, so the in-circuit consequence of swapping it is purely
    // through the escrow-cipher mask.)
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_X,
        Y_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // S13 fires: r_e = 0 would force R_e = O (identity), collapse S_{a,s} to
    // O, and collapse the escrow shared secret s_esc to 0 -- making every
    // mask a knowable constant function of sigma / op_i. Every r_e-dependent
    // public (R_e = (0, 0) identity-encoding, escrowed_dvk_cipher derived
    // against s_esc = 0, and the auditor channel ciphertexts derived against
    // s_a_s_x = 0) is recomputed so the `assert(r_e != 0)` check is the only
    // constraint that can catch this -- removing S13 would let the proof
    // verify.
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_esc_zero = ecdh(0, y_op);
    let escrowed_dvk_cipher_zero = encrypt_esc_dvk(dvk, s_esc_zero, OP_I);
    let s_a_s_x_zero = ecdh(0, k_aud_s);
    let m_s_zero = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_zero, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_A,
        0,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        0,
        escrowed_dvk_cipher_zero,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        0,
        0,
        V_A + m_s_zero[0],
        V_NEW + m_s_zero[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: S_a1 fails because scalar_mul(r_e+1, H)
    // no longer equals the canonical (R_E_X, R_E_Y). R_e itself is left
    // canonical (so S_a1 catches the mismatch) but every other r_e-dependent
    // public (escrowed_dvk_cipher, both auditor channel ciphertexts) is
    // recomputed against r_e+1 so S12(c) / S_a4 / S_a5 cannot coincidentally
    // fire. escrowed_dvk_r_x is also left canonical at R_E_X so S12(a) keeps
    // matching the public R_e -- the only catchable mismatch is S_a1's
    // derived point.
    let r_e_bad: Field = R_E + 1;
    let vk = vk_from_sk(SK, ADDR_F);
    let dvk = dvk_from_vk_op(vk, OP_I);
    let y_op = EmbeddedCurvePoint { x: Y_OP_X, y: Y_OP_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_esc_bad = ecdh(r_e_bad, y_op);
    let escrowed_dvk_cipher_bad = encrypt_esc_dvk(dvk, s_esc_bad, OP_I);
    let s_a_s_x_bad = ecdh(r_e_bad, k_aud_s);
    let m_s_bad = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_bad, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_A,
        r_e_bad,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        escrowed_dvk_cipher_bad,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_A + m_s_bad[0],
        V_NEW + m_s_bad[1],
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    // (1, 2) is off-curve. The on-curve check on K_aud_s fires before S_a2
    // ever consumes it.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        1,
        2,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    // K_aud_s = identity collapses ECDH; the on-curve check rejects identity
    // for keys.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        0,
        0,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_s() {
    // K_aud_s replaced by a valid but different on-curve key (Y_X, Y_Y).
    // Shared secret changes, owner-channel masks change, S_a4 fails.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        Y_X,
        Y_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_s() {
    // S_a4 fires.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S + 1,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde_aud_s() {
    // S_a5 fires.
    run_main(
        SK,
        V,
        R,
        V_A,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        Y_OP_X,
        Y_OP_Y,
        OP_I,
        ADDR_F,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_A_X,
        C_A_Y,
        R_E_X,
        ESCROWED_DVK_CIPHER,
        B_TILDE,
        A_TILDE,
        SIGMA,
        SIGMA_A,
        R_E_X,
        R_E_Y,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S + 1,
    );
}
