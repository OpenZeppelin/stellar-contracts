use crate::main;
use stellar_confidential_lib::{
    commit, derive_spend_r, derive_transfer_blind, domain, ecdh, encrypt_amount, encrypt_balance, H,
    scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SK / ADDR_F / SIGMA / V / R / V_TRANSFER / R_E reuse the lib's pinned vectors
// (lib/testdata) so that Y, C_spend, vk, r_new, b_tilde, r_transfer, v_tilde, and
// the recipient/sender-channel masks are all derivable from -- and pinned by
// -- the lib's `fixtures_match_testdata` test. V_NEW = V - V_TRANSFER = 900 keeps
// the happy path on a non-trivial v - v_transfer while still satisfying T4.
// VK_B / K_AUD_R_SCALAR / K_AUD_S_SCALAR are local placeholder scalars used
// to materialize on-curve PVK_B / K_aud_r / K_aud_s points; any non-zero
// scalar would do, these are pinned by `transfer_fixtures_match_lib`.
global SK: Field = 0xdead;
global ADDR_F: Field = 0xbeef;
global SIGMA: Field = 0x01;
global V: Field = 1000;
global R: Field = 42;
global V_TRANSFER: Field = 100;
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
global C_TRANSFER_X: Field = 0x26677e8f24cbbc929b8be4a8d470d4a0e54a3c8a351ceef295e6b99b2898ed1d;
global C_TRANSFER_Y: Field = 0x089153eeedb04e49b206f7121341fdcb842a6ca19fb0f938167834dd10d42a97;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global V_TILDE: Field = 0x0b3b7be1cd27249ec6b32b4ecb840079e0354b8675e94aade6519e5428473ffa;
global B_TILDE: Field = 0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c5738;
global V_TILDE_AUD_R: Field =
    0x1150b1568e3047a5d263b9c24b00f49ca3971880e2fd09e9e6448207136bbb41;
global R_TILDE_AUD_R: Field =
    0x16cb31d142cf3005f8bd95ab3f59c3bda0da0de7e1d2ac16b4d038548a02b364;
global V_TILDE_AUD_S: Field =
    0x0d0a3aa77960977737f4355093d4672ea9f95556fabf63bda0913ebef52c42cf;
global B_TILDE_AUD_S: Field =
    0x288ff48898f218dbba2a5c9546d0986af3257a4a539e23e4f35c404d6720c144;

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
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
    let c_transfer = commit(V_TRANSFER, r_transfer);
    let v_tilde = encrypt_amount(V_TRANSFER, s_x, SIGMA);

    let r_new = derive_spend_r(vk, SIGMA);
    let c_spend_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);

    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let v_tilde_aud_r = V_TRANSFER + m_r[0];
    let r_tilde_aud_r = r_transfer + m_r[1];

    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    let v_tilde_aud_s = V_TRANSFER + m_s[0];
    let b_tilde_aud_s = V_NEW + m_s[1];

    let pvkx = pvk_b.x;
    let pvky = pvk_b.y;
    let kr_x = k_aud_r.x;
    let kr_y = k_aud_r.y;
    let ks_x = k_aud_s.x;
    let ks_y = k_aud_s.y;
    let csnx = c_spend_new.x;
    let csny = c_spend_new.y;
    let c_transfer_x = c_transfer.x;
    let c_transfer_y = c_transfer.y;
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
    println(f"C_TRANSFER_X = {c_transfer_x}");
    println(f"C_TRANSFER_Y = {c_transfer_y}");
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
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
    let c_transfer = commit(V_TRANSFER, r_transfer);
    let v_tilde = encrypt_amount(V_TRANSFER, s_x, SIGMA);

    assert(y.x == Y_X);
    assert(y.y == Y_Y);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
    assert(pvk_b.x == PVK_B_X);
    assert(pvk_b.y == PVK_B_Y);
    assert(c_new.x == C_SPEND_NEW_X);
    assert(c_new.y == C_SPEND_NEW_Y);
    assert(b_tilde == B_TILDE);
    assert(c_transfer.x == C_TRANSFER_X);
    assert(c_transfer.y == C_TRANSFER_Y);
    assert(v_tilde == V_TILDE);
}

#[test]
fn transfer_auditor_fixtures_match_lib() {
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let pvk_b = scalar_mul(VK_B, H);
    let s_x = ecdh(R_E, pvk_b);
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
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
    assert(V_TRANSFER + m_r[0] == V_TILDE_AUD_R);
    assert(r_transfer + m_r[1] == R_TILDE_AUD_R);
    assert(V_TRANSFER + m_s[0] == V_TILDE_AUD_S);
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
    v_transfer_in: Field,
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
    c_transfer_x_in: Field,
    c_transfer_y_in: Field,
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
        v_transfer_in,
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
        c_transfer_x_in,
        c_transfer_y_in,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
    // Boundary: v_transfer = v, so v_new = 0. C_spend' commits to (0, r_new) which
    // is a valid commitment to zero. b_tilde and b_tilde_aud_s both bind to 0
    // (plus mask) so the sender's auditor sees the post-transfer balance
    // reach zero.
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
    let c_transfer = commit(V, r_transfer);
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde,
        SIGMA,
        V + m_r[0],
        r_transfer + m_r[1],
        V + m_s[0],
        0 + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_under_funded_transfer() {
    // v_transfer > v: v - v_transfer underflows in the field to a value >= 2^128. T4 fires.
    // Every downstream public is recomputed against v_transfer_too_large so the
    // single firing constraint is T4's v_new range check.
    let v_transfer_too_large: Field = V + 1;
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
    let c_transfer = commit(v_transfer_too_large, r_transfer);
    let v_tilde = encrypt_amount(v_transfer_too_large, s_x, SIGMA);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_invalid = commit(V - v_transfer_too_large, r_new);
    let b_tilde_invalid = encrypt_balance(V - v_transfer_too_large, vk, SIGMA);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA);
    run_main(
        SK,
        V,
        R,
        v_transfer_too_large,
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde_invalid,
        SIGMA,
        v_transfer_too_large + m_r[0],
        r_transfer + m_r[1],
        v_transfer_too_large + m_s[0],
        (V - v_transfer_too_large) + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_out_of_range() {
    // 2^127 is exactly the boundary T4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). With v_huge - V_TRANSFER = 2^127 - 100 still in [0, 2^127),
    // the v_new range check passes; only the `v.assert_max_bit_size<127>()`
    // sub-check of T4 fires. C_spend, c_spend_new, b_tilde, and b_tilde_aud_s
    // are all recomputed against v_huge / v_new_huge so T3 / T11 / T12 / T_a8
    // cannot coincidentally fire.
    let v_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = v_huge - V_TRANSFER;
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
fn rejects_v_transfer_out_of_range() {
    // v_transfer = 2^127: T4's range check on v_transfer fires. v_new = V - 2^127
    // field-underflows so the v_new range check would also fire, but both
    // belong to T4. All downstream publics that depend on v_transfer (C_transfer,
    // v_tilde, v_tilde_aud_r, v_tilde_aud_s) and on v_new (c_spend_new,
    // b_tilde, b_tilde_aud_s) are recomputed so the only constraints that
    // fail are T4's two range sub-checks.
    let v_transfer_huge: Field = 0x80000000000000000000000000000000;
    let v_new_huge: Field = V - v_transfer_huge;
    let vk = vk_from_sk(SK, ADDR_F);
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_transfer = derive_transfer_blind(s_x, SIGMA);
    let c_transfer = commit(v_transfer_huge, r_transfer);
    let v_tilde = encrypt_amount(v_transfer_huge, s_x, SIGMA);
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
        v_transfer_huge,
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        b_tilde,
        SIGMA,
        v_transfer_huge + m_r[0],
        r_transfer + m_r[1],
        v_transfer_huge + m_s[0],
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
    // T3 fires: commit(V+1, R) != C_spend. v_new = (V+1) - V_TRANSFER = 901 cascades
    // into T11 / T12 / T_a8; recompute c_spend_new, b_tilde, and b_tilde_aud_s
    // against v_new = 901 so only T3 fires.
    let v_bad: Field = V + 1;
    let v_new_bad: Field = v_bad - V_TRANSFER;
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
    // recipient's key). S = r_e * PVK_B changes, so r_transfer and v_tilde drift
    // and T8 fails (C_transfer no longer matches commit(v_transfer, r_transfer_new)).
    run_main(
        SK,
        V,
        R,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
fn rejects_poisoned_c_transfer() {
    // Anti-poisoning (Section 5.4): a sender that commits C_transfer with arbitrary
    // blinding -- here r_transfer' = canonical r_transfer + 1 -- desynchronizes the
    // recipient's accumulated blinding. T7 + T8 together close this attack:
    // r_transfer is uniquely determined by Poseidon2(delta_transfer_blind, s, sigma),
    // so any C_transfer not built from that exact r_transfer fails T8.
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_b);
    let r_transfer_poisoned = derive_transfer_blind(s_x, SIGMA) + 1;
    let c_transfer_poisoned = commit(V_TRANSFER, r_transfer_poisoned);
    run_main(
        SK,
        V,
        R,
        V_TRANSFER,
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
        c_transfer_poisoned.x,
        c_transfer_poisoned.y,
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
    // T9 fires: v_tilde + 1 no longer matches v_transfer + Poseidon2(TRANSFER_AMOUNT, ...).
    run_main(
        SK,
        V,
        R,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
    // (R_e = (0, 0) identity-encoding, C_transfer / v_tilde derived against s_x = 0,
    // and both auditor channels' ciphertexts derived against s_a_{r,s}.x = 0)
    // is recomputed so the `assert(r_e != 0)` check is the only constraint
    // that can catch this -- removing T13 would let the proof verify.
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_zero = ecdh(0, pvk_b);
    let r_transfer_zero = derive_transfer_blind(s_x_zero, SIGMA);
    let c_transfer_zero = commit(V_TRANSFER, r_transfer_zero);
    let v_tilde_zero = encrypt_amount(V_TRANSFER, s_x_zero, SIGMA);
    let s_a_r_x_zero = ecdh(0, k_aud_r);
    let m_r_zero = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_zero, SIGMA);
    let s_a_s_x_zero = ecdh(0, k_aud_s);
    let m_s_zero = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_zero, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_TRANSFER,
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
        c_transfer_zero.x,
        c_transfer_zero.y,
        0,
        0,
        v_tilde_zero,
        B_TILDE,
        SIGMA,
        V_TRANSFER + m_r_zero[0],
        r_transfer_zero + m_r_zero[1],
        V_TRANSFER + m_s_zero[0],
        V_NEW + m_s_zero[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: T6 fails because scalar_mul(r_e+1, H)
    // no longer equals the canonical (R_E_X, R_E_Y). R_e itself is left
    // canonical (so T6 catches the mismatch) but every other r_e-dependent
    // public (C_transfer, v_tilde, both auditor channels) is recomputed against
    // r_e+1 so T8 / T9 / T_a3 / T_a4 / T_a7 / T_a8 cannot coincidentally fire.
    let r_e_bad: Field = R_E + 1;
    let pvk_b = EmbeddedCurvePoint { x: PVK_B_X, y: PVK_B_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_bad = ecdh(r_e_bad, pvk_b);
    let r_transfer_bad = derive_transfer_blind(s_x_bad, SIGMA);
    let c_transfer_bad = commit(V_TRANSFER, r_transfer_bad);
    let v_tilde_bad = encrypt_amount(V_TRANSFER, s_x_bad, SIGMA);
    let s_a_r_x_bad = ecdh(r_e_bad, k_aud_r);
    let m_r_bad = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_bad, SIGMA);
    let s_a_s_x_bad = ecdh(r_e_bad, k_aud_s);
    let m_s_bad = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_bad, SIGMA);
    run_main(
        SK,
        V,
        R,
        V_TRANSFER,
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
        c_transfer_bad.x,
        c_transfer_bad.y,
        R_E_X,
        R_E_Y,
        v_tilde_bad,
        B_TILDE,
        SIGMA,
        V_TRANSFER + m_r_bad[0],
        r_transfer_bad + m_r_bad[1],
        V_TRANSFER + m_s_bad[0],
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
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
