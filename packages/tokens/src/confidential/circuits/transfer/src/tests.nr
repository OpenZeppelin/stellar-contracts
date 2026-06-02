use crate::main;
use stellar_confidential_lib::{
    commit, derive_spend_r, derive_tx_blind, domain, ecdh, encrypt_amount, encrypt_balance, H,
    scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SK / ADDR_F / SIGMA / V / R / V_TX / R_E reuse the lib's pinned vectors
// (lib/testdata) so that Y, C_spend, vk, r_new, b_tilde, r_tx, v_tilde, and
// the recipient/sender-channel masks are all derivable from -- and pinned by
// -- the lib's `fixtures_match_testdata` test. V_NEW = V - V_TX = 900 keeps
// the happy path on a non-trivial v - v_tx while still satisfying T4.
// VK_B / K_AUD_R_SCALAR / K_AUD_S_SCALAR are local placeholder scalars used
// to materialize on-curve PVK_B / K_aud_r / K_aud_s points; any non-zero
// scalar would do, these are pinned by `transfer_fixtures_match_lib`.
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global SIGMA: Field = 0x01;
global V: Field = 1000;
global R: Field = 42;
global V_TX: Field = 100;
global V_NEW: Field = 900;
global R_E: Field = 0xfeedface;

// Placeholder recipient viewing-key scalar; PVK_B = VK_B * H.
global VK_B: Field = 0xfeed;
// Placeholder auditor secret scalars; K_aud_{r,s} = scalar * H. K_aud_s reuses
// the withdraw fixture's auditor scalar so the two circuits share the same
// sender-auditor key in cross-circuit tooling; K_aud_r is distinct so the
// dual-channel separation is exercised by the happy-path test.
global K_AUD_R_SCALAR: Field = 0xc0ffee01;
global K_AUD_S_SCALAR: Field = 0xc0ffee;

// Y = sk*H (= lib `scalar_mul_Y` fixture).
global Y_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;

// C_spend = commit(1000, 42) (= lib `commit` fixture).
global C_SPEND_X: Field =
    0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05;
global C_SPEND_Y: Field =
    0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e;

// All values below are pinned by `transfer_fixtures_match_lib` and
// `transfer_auditor_fixtures_match_lib`, which re-derive them from the lib
// primitives and assert the values declared here -- if either side drifts
// the test fails immediately. They are emitted by `print_fixtures`.
global PVK_B_X: Field = 0x176667da789ef2e73e193b32ff0198687cadb0e5d1843fa8ca3140cbf352aaf0;
global PVK_B_Y: Field = 0x017ad8f680161d399bcb7034881f3cd6288f99138d51bddf3331a92257698b85;
global K_AUD_R_X: Field = 0x07782f9cf649fcc3690b758cd826426732ae8dad766ec62860ca0330d8b0e3fe;
global K_AUD_R_Y: Field = 0x26a06cc59b1da39f1c50a1deae9bb5289b36c0350a9942ac4bf5a6079717dbd8;
global K_AUD_S_X: Field = 0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_S_Y: Field = 0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;
global C_SPEND_NEW_X: Field =
    0x11aca8a40b3357f0b9f87a5c4a9ca1b6599f9a70cc9d29b7f49fe2fc2bd16477;
global C_SPEND_NEW_Y: Field =
    0x0e4f3b326ff5b603ea400a3b0e31a95472db13ef487524952ecd7387719fcde4;
global C_TX_X: Field = 0x1679ef51dbb01793592348ed9dcc453f82aba5d4777d786ea09979e06b965dfc;
global C_TX_Y: Field = 0x1ec2eb1cc4b0cfd172637f34166074f3e15e6620d814cc22bae87eeb2a40270d;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global V_TILDE: Field = 0x2861fc81fda926afd33517cb5509df867f324abf81c82ab9d7eca9a94a16f07b;
global B_TILDE: Field = 0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c5738;
global V_TILDE_AUD_R: Field =
    0x2d80e4f8a46472bb837956b13028b7d95979a8f5e0f657c50a7d6721f9a19f66;
global R_TILDE_AUD_R: Field =
    0x0b9fb0b33e05f9c93da3802075248e25b59f1b2416c997ffbedd6cd71e1c1385;
global V_TILDE_AUD_S: Field =
    0x0a607feb31e56e89ed94a3e1dc3811d3d23aaf10b2f27874ce670878b9f6ee19;
global B_TILDE_AUD_S: Field =
    0x0b586defafdf4583d0f338855fdf8391102adaac4634a3b80594aea6f788e1cb;

#[test]
fn print_fixtures() {
    // One-shot harness: prints every derived public input for the chosen
    // fixture tuple. Run with
    // `nargo test --package circuit_transfer print_fixtures --show-output`
    // and paste the values into the globals above.
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = scalar_mul(VK_B, H);
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);

    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let c_tx = commit(V_TX, r_tx);
    let v_tilde = encrypt_amount(V_TX, s_x, SIGMA);

    let r_new = derive_spend_r(vk, SIGMA);
    let c_spend_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);

    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let v_tilde_aud_r = V_TX + m_r[0];
    let r_tilde_aud_r = r_tx + m_r[1];

    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = V_TX + m_s[0];
    let b_tilde_aud_s = V_NEW + m_s[1];

    let pvkx = pvk_b.x;
    let pvky = pvk_b.y;
    let kr_x = k_aud_r.x;
    let kr_y = k_aud_r.y;
    let ks_x = k_aud_s.x;
    let ks_y = k_aud_s.y;
    let csnx = c_spend_new.x;
    let csny = c_spend_new.y;
    let ctxx = c_tx.x;
    let ctxy = c_tx.y;
    let rex = r_e_pt.x;
    let rey = r_e_pt.y;
    println(f"PVK_B_X = {pvkx}");
    println(f"PVK_B_Y = {pvky}");
    println(f"K_AUD_R_X = {kr_x}");
    println(f"K_AUD_R_Y = {kr_y}");
    println(f"K_AUD_S_X = {ks_x}");
    println(f"K_AUD_S_Y = {ks_y}");
    println(f"C_SPEND_NEW_X = {csnx}");
    println(f"C_SPEND_NEW_Y = {csny}");
    println(f"C_TX_X = {ctxx}");
    println(f"C_TX_Y = {ctxy}");
    println(f"R_E_X = {rex}");
    println(f"R_E_Y = {rey}");
    println(f"V_TILDE = {v_tilde}");
    println(f"B_TILDE = {b_tilde}");
    println(f"V_TILDE_AUD_R = {v_tilde_aud_r}");
    println(f"R_TILDE_AUD_R = {r_tilde_aud_r}");
    println(f"V_TILDE_AUD_S = {v_tilde_aud_s}");
    println(f"B_TILDE_AUD_S = {b_tilde_aud_s}");
}

#[test]
fn transfer_fixtures_match_lib() {
    let vk = vk_from_sk(SK, ADDR_F);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let y = scalar_mul(SK, H);
    let c_spend = commit(V, R);
    let pvk_b = scalar_mul(VK_B, H);
    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let c_tx = commit(V_TX, r_tx);
    let v_tilde = encrypt_amount(V_TX, s_x, SIGMA);

    assert(y.x == Y_X);
    assert(y.y == Y_Y);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
    assert(pvk_b.x == PVK_B_X);
    assert(pvk_b.y == PVK_B_Y);
    assert(c_new.x == C_SPEND_NEW_X);
    assert(c_new.y == C_SPEND_NEW_Y);
    assert(b_tilde == B_TILDE);
    assert(c_tx.x == C_TX_X);
    assert(c_tx.y == C_TX_Y);
    assert(v_tilde == V_TILDE);
}

#[test]
fn transfer_auditor_fixtures_match_lib() {
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let pvk_b = scalar_mul(VK_B, H);
    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);

    assert(k_aud_r.x == K_AUD_R_X);
    assert(k_aud_r.y == K_AUD_R_Y);
    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(V_TX + m_r[0] == V_TILDE_AUD_R);
    assert(r_tx + m_r[1] == R_TILDE_AUD_R);
    assert(V_TX + m_s[0] == V_TILDE_AUD_S);
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
    v_tx_in: Field,
    r_e_in: Field,
    c_spend_x_in: Field,
    c_spend_y_in: Field,
    y_x_in: Field,
    y_y_in: Field,
    addr_f_in: Field,
    pvk_b_x_in: Field,
    pvk_b_y_in: Field,
    k_aud_r_x_in: Field,
    k_aud_r_y_in: Field,
    k_aud_s_x_in: Field,
    k_aud_s_y_in: Field,
    c_spend_new_x_in: Field,
    c_spend_new_y_in: Field,
    c_tx_x_in: Field,
    c_tx_y_in: Field,
    r_e_x_in: Field,
    r_e_y_in: Field,
    v_tilde_in: Field,
    b_tilde_in: Field,
    sigma_in: Field,
    v_tilde_aud_r_in: Field,
    r_tilde_aud_r_in: Field,
    v_tilde_aud_s_in: Field,
    b_tilde_aud_s_in: Field,
) {
    main(
        sk_in,
        v_in,
        r_in,
        v_tx_in,
        r_e_in,
        c_spend_x_in,
        c_spend_y_in,
        y_x_in,
        y_y_in,
        pvk_b_x_in,
        pvk_b_y_in,
        addr_f_in,
        k_aud_r_x_in,
        k_aud_r_y_in,
        k_aud_s_x_in,
        k_aud_s_y_in,
        c_spend_new_x_in,
        c_spend_new_y_in,
        c_tx_x_in,
        c_tx_y_in,
        r_e_x_in,
        r_e_y_in,
        v_tilde_in,
        b_tilde_in,
        sigma_in,
        v_tilde_aud_r_in,
        r_tilde_aud_r_in,
        v_tilde_aud_s_in,
        b_tilde_aud_s_in,
    );
}

fn run_fixture() {
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test]
fn matches_fixture() {
    run_fixture();
}

#[test]
fn full_balance_transfer() {
    // Boundary: v_tx = v, so v_new = 0. C_spend' commits to (0, r_new) which
    // is a valid commitment to zero. b_tilde and b_tilde_aud_s both bind to 0
    // (plus mask) so the sender's auditor sees the post-transfer balance
    // reach zero.
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let c_tx = commit(V, r_tx);
    let v_tilde = encrypt_amount(V, s_x, SIGMA);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(0, r_new);
    let b_tilde = encrypt_balance(0, vk, SIGMA);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
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
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde,
        SIGMA,
        V + m_r[0],
        r_tx + m_r[1],
        V + m_s[0],
        0 + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_under_funded_transfer() {
    // v_tx > v: v - v_tx underflows in the field to a value >= 2^128. T4 fires.
    // Every downstream public is recomputed against v_tx_too_large so the
    // single firing constraint is T4's v_new range check.
    let v_tx_too_large: Field = V + 1;
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let c_tx = commit(v_tx_too_large, r_tx);
    let v_tilde = encrypt_amount(v_tx_too_large, s_x, SIGMA);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_invalid = commit(V - v_tx_too_large, r_new);
    let b_tilde_invalid = encrypt_balance(V - v_tx_too_large, vk, SIGMA);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        v_tx_too_large,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_invalid.x,
        c_new_invalid.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde_invalid,
        SIGMA,
        v_tx_too_large + m_r[0],
        r_tx + m_r[1],
        v_tx_too_large + m_s[0],
        (V - v_tx_too_large) + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_out_of_range() {
    // 2^127 is exactly the boundary T4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). With v_huge - V_TX = 2^127 - 100 still in [0, 2^127),
    // the v_new range check passes; only the `v.assert_max_bit_size<127>()`
    // sub-check of T4 fires. C_spend, c_spend_new, b_tilde, and b_tilde_aud_s
    // are all recomputed against v_huge / v_new_huge so T3 / T11 / T12 / T_a8
    // cannot coincidentally fire.
    let v_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = v_huge - V_TX;
    let vk = vk_from_sk(SK, ADDR_F);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let c_spend = commit(v_huge, R);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_huge, r_new);
    let b_tilde = encrypt_balance(v_new_huge, vk, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        v_huge,
        R,
        V_TX,
        R_E,
        c_spend.x,
        c_spend.y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        b_tilde,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_tx_out_of_range() {
    // v_tx = 2^127: T4's range check on v_tx fires. v_new = V - 2^127
    // field-underflows so the v_new range check would also fire, but both
    // belong to T4. All downstream publics that depend on v_tx (C_tx,
    // v_tilde, v_tilde_aud_r, v_tilde_aud_s) and on v_new (c_spend_new,
    // b_tilde, b_tilde_aud_s) are recomputed so the only constraints that
    // fail are T4's two range sub-checks.
    let v_tx_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = V - v_tx_huge;
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_tx = derive_tx_blind(s_x, SIGMA);
    let c_tx = commit(v_tx_huge, r_tx);
    let v_tilde = encrypt_amount(v_tx_huge, s_x, SIGMA);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(v_new_huge, r_new);
    let b_tilde = encrypt_balance(v_new_huge, vk, SIGMA);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        v_tx_huge,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new.x,
        c_new.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde,
        SIGMA,
        v_tx_huge + m_r[0],
        r_tx + m_r[1],
        v_tx_huge + m_s[0],
        v_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_sk() {
    // sk + 1 != sk: T1 fails because scalar_mul(sk+1, H) no longer matches Y.
    // vk is rebound to (sk+1, addr_f) and the vk-dependent publics
    // (c_spend_new, b_tilde) are recomputed under the new vk so T11 / T12
    // cannot fire alongside T1. b_tilde_aud_s does not depend on vk so its
    // canonical value still satisfies T_a8.
    let vk_bad = vk_from_sk(SK + 1, ADDR_F);
    let r_new_bad = derive_spend_r(vk_bad, SIGMA);
    let c_new_bad = commit(V_NEW, r_new_bad);
    let b_tilde_bad = encrypt_balance(V_NEW, vk_bad, SIGMA);
    run_main(
        SK + 1,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        b_tilde_bad,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_balance_opening() {
    // T3 fires: commit(V+1, R) != C_spend. v_new = (V+1) - V_TX = 901 cascades
    // into T11 / T12 / T_a8; recompute c_spend_new, b_tilde, and b_tilde_aud_s
    // against v_new = 901 so only T3 fires.
    let v_bad: Field = V + 1;
    let v_new_bad: Field = v_bad - V_TX;
    let vk = vk_from_sk(SK, ADDR_F);
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_bad = commit(v_new_bad, r_new);
    let b_tilde_bad = encrypt_balance(v_new_bad, vk, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        v_bad,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        b_tilde_bad,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_new_bad + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_addr_f() {
    // The circuit binds vk to addr_f via T2 but doesn't assert addr_f directly;
    // the binding is visible only through vk-derived publics (T11, T12). With
    // addr_f_bad, recompute c_spend_new from vk_bad so T11 passes -- leaving
    // b_tilde canonical, where the addr_f-binding catches the mismatch via T12.
    // (Recomputing both, as in `rejects_wrong_sk`, would make the proof
    // verify: T1 is the load-bearing addr_f-independent check there, and no
    // such constraint exists here.)
    let addr_f_bad: Field = 0xcafe;
    let vk_bad = vk_from_sk(SK, addr_f_bad);
    let r_new_bad = derive_spend_r(vk_bad, SIGMA);
    let c_new_bad = commit(V_NEW, r_new_bad);
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        addr_f_bad,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_new_bad.x,
        c_new_bad.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_recipient_pvk() {
    // PVK_B replaced by Y (a valid on-curve point that isn't the registered
    // recipient's key). S = r_e * PVK_B changes, so r_tx and v_tilde drift
    // and T8 fails (C_tx no longer matches commit(v_tx, r_tx_new)).
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        Y_X,
        Y_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_poisoned_c_tx() {
    // Anti-poisoning (Section 5.4): a sender that commits C_tx with arbitrary
    // blinding -- here r_tx' = canonical r_tx + 1 -- desynchronizes the
    // recipient's accumulated blinding. T7 + T8 together close this attack:
    // r_tx is uniquely determined by Poseidon2(delta_tx_blind, S.x, sigma),
    // so any C_tx not built from that exact r_tx fails T8.
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_tx_poisoned = derive_tx_blind(s_x, SIGMA) + 1;
    let c_tx_poisoned = commit(V_TX, r_tx_poisoned);
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        c_tx_poisoned.x,
        c_tx_poisoned.y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde() {
    // T9 fires: v_tilde + 1 no longer matches v_tx + Poseidon2(TX_AMOUNT, ...).
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE + 1,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde() {
    // T12 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE + 1,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_spend_new() {
    // T11 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X + 1,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // T13 fires: r_e = 0 would force R_e = O (identity) and collapse every
    // ECDH in this transfer (S, S_{a,r}, S_{a,s} all become O), making every
    // mask a knowable constant function of sigma. Every r_e-dependent public
    // (R_e = (0, 0) identity-encoding, C_tx / v_tilde derived against s_x = 0,
    // and both auditor channels' ciphertexts derived against s_a_{r,s}.x = 0)
    // is recomputed so the `assert(r_e != 0)` check is the only constraint
    // that can catch this -- removing T13 would let the proof verify.
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_zero = ecdh(0, pvk_b);
    let r_tx_zero = derive_tx_blind(s_x_zero, SIGMA);
    let c_tx_zero = commit(V_TX, r_tx_zero);
    let v_tilde_zero = encrypt_amount(V_TX, s_x_zero, SIGMA);
    let s_a_r_x_zero = ecdh(0, k_aud_r);
    let m_r_zero = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_zero, SIGMA);
    let s_a_s_x_zero = ecdh(0, k_aud_s);
    let m_s_zero = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_zero, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_TX,
        0,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        c_tx_zero.x,
        c_tx_zero.y,
        0,
        0,
        v_tilde_zero,
        B_TILDE,
        SIGMA,
        V_TX + m_r_zero[0],
        r_tx_zero + m_r_zero[1],
        V_TX + m_s_zero[0],
        V_NEW + m_s_zero[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: T6 fails because scalar_mul(r_e+1, H)
    // no longer equals the canonical (R_E_X, R_E_Y). R_e itself is left
    // canonical (so T6 catches the mismatch) but every other r_e-dependent
    // public (C_tx, v_tilde, both auditor channels) is recomputed against
    // r_e+1 so T8 / T9 / T_a3 / T_a4 / T_a7 / T_a8 cannot coincidentally fire.
    let r_e_bad: Field = R_E + 1;
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_bad = ecdh(r_e_bad, pvk_b);
    let r_tx_bad = derive_tx_blind(s_x_bad, SIGMA);
    let c_tx_bad = commit(V_TX, r_tx_bad);
    let v_tilde_bad = encrypt_amount(V_TX, s_x_bad, SIGMA);
    let s_a_r_x_bad = ecdh(r_e_bad, k_aud_r);
    let m_r_bad = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_bad, SIGMA);
    let s_a_s_x_bad = ecdh(r_e_bad, k_aud_s);
    let m_s_bad = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_bad, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_TX,
        r_e_bad,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        c_tx_bad.x,
        c_tx_bad.y,
        R_E_X,
        R_E_Y,
        v_tilde_bad,
        B_TILDE,
        SIGMA,
        V_TX + m_r_bad[0],
        r_tx_bad + m_r_bad[1],
        V_TX + m_s_bad[0],
        V_NEW + m_s_bad[1],
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_r() {
    // (1, 2) is off-curve. The on-curve check on K_aud_r fires before T_a1
    // ever consumes it.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        1,
        2,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_r() {
    // K_aud_r = identity collapses ECDH; the on-curve check rejects identity
    // for keys.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        0,
        0,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        1,
        2,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        0,
        0,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_r() {
    // K_aud_r replaced by a valid but different on-curve key (Y_X, Y_Y).
    // Shared secret changes, recipient-channel masks change, T_a3 fails.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        Y_X,
        Y_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_s() {
    // K_aud_s replaced by a valid but different on-curve key (Y_X, Y_Y).
    // Shared secret changes, sender-channel masks change, T_a7 fails.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        Y_X,
        Y_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_r() {
    // T_a3 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R + 1,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_r_tilde_aud_r() {
    // T_a4 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R + 1,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_s() {
    // T_a7 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S + 1,
        B_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde_aud_s() {
    // T_a8 fires.
    run_main(
        SK,
        V,
        R,
        V_TX,
        R_E,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        ADDR_F,
        PVK_B_X,
        PVK_B_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        B_TILDE,
        SIGMA,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        B_TILDE_AUD_S + 1,
    );
}
