use crate::main;
use stellar_confidential_lib::{
    commit, derive_allow_r, derive_tx_blind, domain, ecdh, encrypt_allowance, encrypt_amount, H,
    scalar_mul, sponge_squeeze_2,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SIGMA_A is the channel nonce reused across the allowance Poseidon (O3),
// the recipient ECDH chain (O7 / O9), and both auditor sponges (O_a2 / O_a6)
// -- see main.nr "Channel-nonce reuse". SIGMA_A_NEW is the fresh salt for the
// post-transfer allowance derivations only (O10, O12). V_A / V_TX / V_A_NEW
// give a non-trivial `v_a - v_tx` so O4 is exercised on a real subtraction.
//
// Y_OP_X / Y_OP_Y and the auditor / recipient keys share fixture values with
// withdraw/transfer (sk_op = 0xdead, K_aud_s/r and pvk scalars identical to
// transfer) so the three circuits share an ecosystem of pinned curve points
// for cross-circuit tooling. DVK_I is a placeholder field; in production it
// comes from S5 of SetSpender and the spender decrypts it from
// `escrowed_dvk` (Section 7.11).
global SK_OP: Field = 0xdead;
global DVK_I: Field = 0x44767669; // ASCII "Dvki"
global SIGMA_A: Field = 0x01;
global SIGMA_A_NEW: Field = 0x02;
global V_A: Field = 1000;
global V_TX: Field = 100;
global V_A_NEW: Field = 900;
global R_E: Field = 0xfeedface;

// Placeholder recipient viewing-key scalar; PVK_RECIPIENT = VK_RECIPIENT * H.
global VK_RECIPIENT: Field = 0xfeed;
// Placeholder auditor secret scalars; K_aud_{r,s} = scalar * H. Distinct so
// the dual-channel separation is exercised by the happy path; both shared
// with the transfer fixtures for cross-circuit tooling consistency.
global K_AUD_R_SCALAR: Field = 0xc0ffee01;
global K_AUD_S_SCALAR: Field = 0xc0ffee;

// Y_op = sk_op * H (= lib `scalar_mul_Y` fixture for sk = 0xdead).
global Y_OP_X: Field = 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646;
global Y_OP_Y: Field = 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b;

// All values below are pinned by `spender_transfer_fixtures_match_lib` and
// `spender_transfer_auditor_fixtures_match_lib`, which re-derive them from
// the lib primitives and assert the values declared here -- if either side
// drifts the test fails immediately. They are emitted by `print_fixtures`.
global R_A: Field = 0x1528d8006892ad6b484d6f18381c687304d6b4cb9a215992918dd601ad4acf9d;
global C_A_X: Field = 0x1d8739d710d3cc5188faa1e64058cb4986513f46edbbe17c33e2a25da2bb657f;
global C_A_Y: Field = 0x142b6a3fa6772624471dd3e938bbff4a58a17f8bee976a5fcc949c8f487ec209;
global PVK_RECIPIENT_X: Field =
    0x176667da789ef2e73e193b32ff0198687cadb0e5d1843fa8ca3140cbf352aaf0;
global PVK_RECIPIENT_Y: Field =
    0x017ad8f680161d399bcb7034881f3cd6288f99138d51bddf3331a92257698b85;
global K_AUD_R_X: Field = 0x07782f9cf649fcc3690b758cd826426732ae8dad766ec62860ca0330d8b0e3fe;
global K_AUD_R_Y: Field = 0x26a06cc59b1da39f1c50a1deae9bb5289b36c0350a9942ac4bf5a6079717dbd8;
global K_AUD_S_X: Field = 0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_S_Y: Field = 0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;
global C_A_NEW_X: Field = 0x2d19aa5933ba41ebadb28d4c784ca4e405a1a6efcb5cf15e2c8d5e9a0552f178;
global C_A_NEW_Y: Field = 0x0214719083236d5ecfc551fbc2b6d04fa8c95b8986fcfc7bbb8c0ff3cecd3dd1;
global C_TX_X: Field = 0x1679ef51dbb01793592348ed9dcc453f82aba5d4777d786ea09979e06b965dfc;
global C_TX_Y: Field = 0x1ec2eb1cc4b0cfd172637f34166074f3e15e6620d814cc22bae87eeb2a40270d;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global V_TILDE: Field = 0x2861fc81fda926afd33517cb5509df867f324abf81c82ab9d7eca9a94a16f07b;
global A_TILDE_NEW: Field =
    0x1d2e286a8d510a7c4164d0142ceea3202a957d49248426435f311a348f681147;
global V_TILDE_AUD_R: Field =
    0x2d80e4f8a46472bb837956b13028b7d95979a8f5e0f657c50a7d6721f9a19f66;
global R_TILDE_AUD_R: Field =
    0x0b9fb0b33e05f9c93da3802075248e25b59f1b2416c997ffbedd6cd71e1c1385;
global V_TILDE_AUD_S: Field =
    0x0a607feb31e56e89ed94a3e1dc3811d3d23aaf10b2f27874ce670878b9f6ee19;
global A_TILDE_AUD_S: Field =
    0x0b586defafdf4583d0f338855fdf8391102adaac4634a3b80594aea6f788e1cb;

#[test]
fn print_fixtures() {
    // One-shot harness: prints every derived public input for the chosen
    // fixture tuple. Run with
    // `nargo test --package circuit_spender_transfer print_fixtures --show-output`
    // and paste the values into the globals above.
    let y_op = scalar_mul(SK_OP, H);
    let r_a = derive_allow_r(DVK_I, SIGMA_A);
    let c_a = commit(V_A, r_a);

    let pvk_recipient = scalar_mul(VK_RECIPIENT, H);
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);

    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(V_TX, r_tx);
    let v_tilde = encrypt_amount(V_TX, s_x, SIGMA_A);

    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(V_A_NEW, r_a_new);
    let a_tilde_new = encrypt_allowance(V_A_NEW, DVK_I, SIGMA_A_NEW);

    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let v_tilde_aud_r = V_TX + m_r[0];
    let r_tilde_aud_r = r_tx + m_r[1];

    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);
    let v_tilde_aud_s = V_TX + m_s[0];
    let a_tilde_aud_s = V_A_NEW + m_s[1];

    let yopx = y_op.x;
    let yopy = y_op.y;
    let cax = c_a.x;
    let cay = c_a.y;
    let pvkx = pvk_recipient.x;
    let pvky = pvk_recipient.y;
    let kr_x = k_aud_r.x;
    let kr_y = k_aud_r.y;
    let ks_x = k_aud_s.x;
    let ks_y = k_aud_s.y;
    let canx = c_a_new.x;
    let cany = c_a_new.y;
    let ctxx = c_tx.x;
    let ctxy = c_tx.y;
    let rex = r_e_pt.x;
    let rey = r_e_pt.y;
    println(f"Y_OP_X = {yopx}");
    println(f"Y_OP_Y = {yopy}");
    println(f"R_A = {r_a}");
    println(f"C_A_X = {cax}");
    println(f"C_A_Y = {cay}");
    println(f"PVK_RECIPIENT_X = {pvkx}");
    println(f"PVK_RECIPIENT_Y = {pvky}");
    println(f"K_AUD_R_X = {kr_x}");
    println(f"K_AUD_R_Y = {kr_y}");
    println(f"K_AUD_S_X = {ks_x}");
    println(f"K_AUD_S_Y = {ks_y}");
    println(f"C_A_NEW_X = {canx}");
    println(f"C_A_NEW_Y = {cany}");
    println(f"C_TX_X = {ctxx}");
    println(f"C_TX_Y = {ctxy}");
    println(f"R_E_X = {rex}");
    println(f"R_E_Y = {rey}");
    println(f"V_TILDE = {v_tilde}");
    println(f"A_TILDE_NEW = {a_tilde_new}");
    println(f"V_TILDE_AUD_R = {v_tilde_aud_r}");
    println(f"R_TILDE_AUD_R = {r_tilde_aud_r}");
    println(f"V_TILDE_AUD_S = {v_tilde_aud_s}");
    println(f"A_TILDE_AUD_S = {a_tilde_aud_s}");
}

#[test]
fn spender_transfer_fixtures_match_lib() {
    let y_op = scalar_mul(SK_OP, H);
    let r_a = derive_allow_r(DVK_I, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let pvk_recipient = scalar_mul(VK_RECIPIENT, H);
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(V_TX, r_tx);
    let v_tilde = encrypt_amount(V_TX, s_x, SIGMA_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(V_A_NEW, r_a_new);
    let a_tilde_new = encrypt_allowance(V_A_NEW, DVK_I, SIGMA_A_NEW);

    assert(y_op.x == Y_OP_X);
    assert(y_op.y == Y_OP_Y);
    assert(r_a == R_A);
    assert(c_a.x == C_A_X);
    assert(c_a.y == C_A_Y);
    assert(pvk_recipient.x == PVK_RECIPIENT_X);
    assert(pvk_recipient.y == PVK_RECIPIENT_Y);
    assert(c_tx.x == C_TX_X);
    assert(c_tx.y == C_TX_Y);
    assert(v_tilde == V_TILDE);
    assert(c_a_new.x == C_A_NEW_X);
    assert(c_a_new.y == C_A_NEW_Y);
    assert(a_tilde_new == A_TILDE_NEW);
}

#[test]
fn spender_transfer_auditor_fixtures_match_lib() {
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let k_aud_s = scalar_mul(K_AUD_S_SCALAR, H);
    let r_e_pt = scalar_mul(R_E, H);
    let pvk_recipient = scalar_mul(VK_RECIPIENT, H);
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    assert(k_aud_r.x == K_AUD_R_X);
    assert(k_aud_r.y == K_AUD_R_Y);
    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(V_TX + m_r[0] == V_TILDE_AUD_R);
    assert(r_tx + m_r[1] == R_TILDE_AUD_R);
    assert(V_TX + m_s[0] == V_TILDE_AUD_S);
    assert(V_A_NEW + m_s[1] == A_TILDE_AUD_S);
}

// `run_main` accepts every public input as a parameter -- nothing baked in.
// Tests that target a specific constraint construct the public tuple so only
// that constraint can fail: tamper the witness or public input under test
// AND recompute every downstream public that depends on it, so no cascaded
// mismatch coincidentally lights up the same `should_fail`.
fn run_main(
    sk_op_in: Field,
    dvk_i_in: Field,
    v_a_in: Field,
    r_a_in: Field,
    v_tx_in: Field,
    r_e_in: Field,
    c_a_x_in: Field,
    c_a_y_in: Field,
    sigma_a_in: Field,
    y_op_x_in: Field,
    y_op_y_in: Field,
    pvk_recipient_x_in: Field,
    pvk_recipient_y_in: Field,
    k_aud_r_x_in: Field,
    k_aud_r_y_in: Field,
    k_aud_s_x_in: Field,
    k_aud_s_y_in: Field,
    c_a_new_x_in: Field,
    c_a_new_y_in: Field,
    c_tx_x_in: Field,
    c_tx_y_in: Field,
    r_e_x_in: Field,
    r_e_y_in: Field,
    v_tilde_in: Field,
    a_tilde_new_in: Field,
    sigma_a_new_in: Field,
    v_tilde_aud_r_in: Field,
    r_tilde_aud_r_in: Field,
    v_tilde_aud_s_in: Field,
    a_tilde_aud_s_in: Field,
) {
    main(
        sk_op_in,
        dvk_i_in,
        v_a_in,
        r_a_in,
        v_tx_in,
        r_e_in,
        c_a_x_in,
        c_a_y_in,
        sigma_a_in,
        y_op_x_in,
        y_op_y_in,
        pvk_recipient_x_in,
        pvk_recipient_y_in,
        k_aud_r_x_in,
        k_aud_r_y_in,
        k_aud_s_x_in,
        k_aud_s_y_in,
        c_a_new_x_in,
        c_a_new_y_in,
        c_tx_x_in,
        c_tx_y_in,
        r_e_x_in,
        r_e_y_in,
        v_tilde_in,
        a_tilde_new_in,
        sigma_a_new_in,
        v_tilde_aud_r_in,
        r_tilde_aud_r_in,
        v_tilde_aud_s_in,
        a_tilde_aud_s_in,
    );
}

fn run_fixture() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test]
fn matches_fixture() {
    run_fixture();
}

#[test]
fn second_transfer_against_same_allowance() {
    // Second happy-path point: same C_a (= same dvk_i, same v_a, same
    // sigma_a) with a different v_tx. Exercises the case where the spender
    // hasn't yet rotated the allowance commitment but produces a fresh
    // sigma_a_new and a fresh r_e on every call. The transfer-side ECDH
    // chain (O5..O9) is fully rederived; only O11/O12 share dvk_i + the new
    // sigma_a_new with the canonical fixture.
    let v_tx_2: Field = 250;
    let v_a_new_2: Field = V_A - v_tx_2; // 750
    let sigma_a_new_2: Field = 0x03;
    let r_e_2: Field = 0xfacecafe;

    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };

    let s_x = ecdh(r_e_2, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(v_tx_2, r_tx);
    let v_tilde_2 = encrypt_amount(v_tx_2, s_x, SIGMA_A);
    let r_a_new_2 = derive_allow_r(DVK_I, sigma_a_new_2);
    let c_a_new_2 = commit(v_a_new_2, r_a_new_2);
    let a_tilde_new_2 = encrypt_allowance(v_a_new_2, DVK_I, sigma_a_new_2);
    let r_e_2_pt = scalar_mul(r_e_2, H);
    let s_a_r_x = ecdh(r_e_2, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let s_a_s_x = ecdh(r_e_2, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_tx_2,
        r_e_2,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new_2.x,
        c_a_new_2.y,
        c_tx.x,
        c_tx.y,
        r_e_2_pt.x,
        r_e_2_pt.y,
        v_tilde_2,
        a_tilde_new_2,
        sigma_a_new_2,
        v_tx_2 + m_r[0],
        r_tx + m_r[1],
        v_tx_2 + m_s[0],
        v_a_new_2 + m_s[1],
    );
}

#[test]
fn full_allowance_transfer() {
    // Boundary: v_tx = v_a, so v_a_new = 0. C_a' commits to (0, r_a_new'),
    // which is a valid commitment to zero. a_tilde' and a_tilde_aud_s both
    // bind to 0 (plus mask) so the owner's auditor sees the post-transfer
    // allowance reach zero. The delegation entry isn't deleted (that's
    // revoke_spender's job, Section 7.9) -- it now escrows zero value.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(V_A, r_tx);
    let v_tilde = encrypt_amount(V_A, s_x, SIGMA_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(0, r_a_new);
    let a_tilde_new = encrypt_allowance(0, DVK_I, SIGMA_A_NEW);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_A,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new.x,
        c_a_new.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new,
        SIGMA_A_NEW,
        V_A + m_r[0],
        r_tx + m_r[1],
        V_A + m_s[0],
        0 + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_sk_op() {
    // O1 fires: scalar_mul(SK_OP + 1, H) != Y_op.
    run_main(
        SK_OP + 1,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_dvk_against_contract() {
    // Contract-binding regression. The SpenderTransfer circuit does NOT carry
    // a vk derivation -- the spender never sees the owner's sk -- so a naive
    // reading might worry that an spender could reuse a proof's witness
    // across contracts. The defense lives in O3.
    //
    // Mechanism (Section 7.8 final paragraph):
    //   * SetSpender derived dvk_i from a contract-specific vk: S2/S5 chain
    //     vk = Poseidon2(VK, sk_owner, addr_f), then dvk_i = Poseidon2(DVK, vk,
    //     op_i). A different `addr_f` produces a different vk, and therefore a
    //     different dvk_i.
    //   * That dvk_i then determined r_a (S6: r_a = Poseidon2(ALLOW_R, dvk_i,
    //     sigma_a)) and thereby C_a (S7).
    //   * On the SpenderTransfer side, C_a and sigma_a are PUBLIC inputs.
    //     O3 forces the prover's witness dvk_i to satisfy r_a ==
    //     Poseidon2(ALLOW_R, dvk_i, sigma_a), and O2 then forces commit(v_a,
    //     r_a) == C_a. The combination pins dvk_i: only the dvk_i that
    //     SetSpender produced under that contract's vk admits an opening of
    //     C_a.
    //
    // This test exercises the failure mode. We keep every public input
    // canonical (so C_a, sigma_a, Y_op come from the original contract) but
    // hand the prover a `dvk_i` from a DIFFERENT contract -- i.e., one that
    // would have been produced if vk had been derived under addr_f_bad rather
    // than addr_f. With a different dvk_i, derive_allow_r(dvk_i_bad, SIGMA_A)
    // yields a different r_a, so the prover-supplied R_A (also derived from
    // dvk_i_bad and supplied to keep O3 internally consistent) no longer
    // opens C_a under commit(V_A, R_A) -- O2 fails. Alternatively if we
    // leave R_A canonical, O3 fails directly. The path here patches the
    // private witness `dvk_i` only and keeps the canonical R_A; that
    // triggers O3 immediately (r_a_derived != R_A).
    //
    // Why O3 is the load-bearing constraint: O3 is the *only* in-circuit
    // tie between the contract-specific dvk_i (chosen at SetSpender) and
    // the rest of the SpenderTransfer witness. Without it the spender
    // could substitute any dvk_i, satisfying O2 with a re-randomization of
    // r_a, and verify a proof against a C_a from a different contract.
    let dvk_i_bad: Field = DVK_I + 1;
    run_main(
        SK_OP,
        dvk_i_bad,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_allowance_opening() {
    // O2 fires: commit(V_A + 1, R_A) != C_a. v_a_new cascades into O11 / O12
    // / O_a8; recompute c_a_new, a_tilde_new, and a_tilde_aud_s against the
    // tampered v_a_new so only O2 fires.
    let v_a_bad: Field = V_A + 1;
    let v_a_new_bad: Field = v_a_bad - V_TX;
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new_bad = commit(v_a_new_bad, r_a_new);
    let a_tilde_new_bad = encrypt_allowance(v_a_new_bad, DVK_I, SIGMA_A_NEW);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        v_a_bad,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new_bad.x,
        c_a_new_bad.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        a_tilde_new_bad,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_a_new_bad + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_a_new_out_of_range() {
    // v_tx > v_a: v_a - v_tx underflows in the field to a value >= 2^128.
    // O4's `v_a_new` range check fires. Every downstream public is
    // recomputed against v_tx_too_large so the single firing constraint is
    // O4's v_a_new range check (the v_a and v_tx individual sub-checks pass:
    // V_A is in range and v_tx_too_large = V_A + 1 = 1001 is also in range).
    let v_tx_too_large: Field = V_A + 1;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(v_tx_too_large, r_tx);
    let v_tilde = encrypt_amount(v_tx_too_large, s_x, SIGMA_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new_invalid = commit(V_A - v_tx_too_large, r_a_new);
    let a_tilde_new_invalid = encrypt_allowance(V_A - v_tx_too_large, DVK_I, SIGMA_A_NEW);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_tx_too_large,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new_invalid.x,
        c_a_new_invalid.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new_invalid,
        SIGMA_A_NEW,
        v_tx_too_large + m_r[0],
        r_tx + m_r[1],
        v_tx_too_large + m_s[0],
        (V_A - v_tx_too_large) + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_a_out_of_range() {
    // 2^127 is exactly the boundary O4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). With v_a_huge - V_TX = 2^127 - 100 still in [0, 2^127),
    // the v_a_new range check passes; only the `v_a.assert_max_bit_size<127>()`
    // sub-check of O4 fires. C_a, c_a_new, and a_tilde_aud_s are all
    // recomputed against v_a_huge / v_a_new_huge so O2 / O11 / O12 / O_a8
    // cannot coincidentally fire. R_A is independent of v_a so it stays
    // canonical.
    let v_a_huge: Field = 0x80000000000000000000000000000000;
    let v_a_new_huge: Field = v_a_huge - V_TX;
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let c_a = commit(v_a_huge, R_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(v_a_new_huge, r_a_new);
    let a_tilde_new = encrypt_allowance(v_a_new_huge, DVK_I, SIGMA_A_NEW);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        v_a_huge,
        R_A,
        V_TX,
        R_E,
        c_a.x,
        c_a.y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new.x,
        c_a_new.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        a_tilde_new,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_a_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_v_tx_out_of_range() {
    // v_tx = 2^127: O4's range check on v_tx fires. Every downstream public
    // that depends on v_tx (C_tx, v_tilde, v_tilde_aud_r, v_tilde_aud_s) and
    // on v_a_new (c_a_new, a_tilde_new, a_tilde_aud_s) is recomputed so the
    // only constraints that fail are O4's range sub-checks.
    let v_tx_huge: Field = 0x80000000000000000000000000000000;
    let v_a_new_huge: Field = V_A - v_tx_huge;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx = derive_tx_blind(s_x, SIGMA_A);
    let c_tx = commit(v_tx_huge, r_tx);
    let v_tilde = encrypt_amount(v_tx_huge, s_x, SIGMA_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(v_a_new_huge, r_a_new);
    let a_tilde_new = encrypt_allowance(v_a_new_huge, DVK_I, SIGMA_A_NEW);
    let s_a_r_x = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, SIGMA_A);
    let s_a_s_x = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_tx_huge,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new.x,
        c_a_new.y,
        c_tx.x,
        c_tx.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new,
        SIGMA_A_NEW,
        v_tx_huge + m_r[0],
        r_tx + m_r[1],
        v_tx_huge + m_s[0],
        v_a_new_huge + m_s[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_recipient_pvk() {
    // PVK_recipient replaced by Y_op (a valid on-curve point that isn't the
    // registered recipient's PVK). S = r_e * PVK_recipient changes, so r_tx
    // and v_tilde drift and O8 fails (C_tx no longer matches commit(v_tx,
    // r_tx_new)).
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        Y_OP_X,
        Y_OP_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_poisoned_c_tx() {
    // Anti-poisoning (Section 5.4): a prover that commits C_tx with arbitrary
    // blinding -- here r_tx + 1 -- desynchronizes the recipient's accumulated
    // blinding. O7 + O8 together close this attack: r_tx is uniquely
    // determined by Poseidon2(delta_tx_blind, S.x, sigma_a), so any C_tx not
    // built from that exact r_tx fails O8.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let s_x = ecdh(R_E, pvk_recipient);
    let r_tx_poisoned = derive_tx_blind(s_x, SIGMA_A) + 1;
    let c_tx_poisoned = commit(V_TX, r_tx_poisoned);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        c_tx_poisoned.x,
        c_tx_poisoned.y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde() {
    // O9 fires: v_tilde + 1 no longer matches v_tx + Poseidon2(TX_AMOUNT, ...).
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE + 1,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

// O10 (`r_a' = Poseidon2(delta_allow_r, dvk_i, sigma_a')`) is exercised
// transitively by the two tests below: a tampered `c_a_new` is detected by
// O11 (which consumes `r_a_new`), and a tampered `a_tilde_new` is detected
// by O12 (which consumes `dvk_i` and `sigma_a'` directly). `dvk_i` is the
// single shared witness between O3 and O10, so isolating O10 with its own
// dedicated negative test would require a second `dvk_i` witness that
// doesn't exist in the circuit.

#[test(should_fail)]
fn rejects_tampered_a_tilde_new() {
    // O12 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW + 1,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_a_new() {
    // O11 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X + 1,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_sigma_a_new_equal_to_sigma_a() {
    // Salt-rotation hardening: the circuit rejects `sigma_a' == sigma_a`
    // even though Section 7.8 does not require it. With `sigma_a' = sigma_a`:
    //   r_a_new = derive_allow_r(dvk_i, sigma_a_new) = r_a
    //   c_a_new = (V_A - V_TX) * G + r_a * H
    //   a_tilde_new = encrypt_allowance(V_A - V_TX, dvk_i, sigma_a_new)
    // All downstream publics are recomputed consistently so the only
    // firing constraint is `assert(sigma_a_new != sigma_a)`.
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A);
    let c_a_new = commit(V_A_NEW, r_a_new);
    let a_tilde_new = encrypt_allowance(V_A_NEW, DVK_I, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        c_a_new.x,
        c_a_new.y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        a_tilde_new,
        SIGMA_A,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // O13 fires: r_e = 0 would force R_e = O (identity) and collapse every
    // ECDH in this transfer (S, S_{a,r}, S_{a,s} all become O), making every
    // mask a knowable constant function of sigma_a. Every r_e-dependent
    // public (R_e = (0, 0) identity-encoding, C_tx / v_tilde derived against
    // s_x = 0, and both auditor channels' ciphertexts derived against
    // s_a_{r,s}.x = 0) is recomputed so the `assert(r_e != 0)` check is the
    // only constraint that can catch this -- removing O13 would let the
    // proof verify.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_zero = ecdh(0, pvk_recipient);
    let r_tx_zero = derive_tx_blind(s_x_zero, SIGMA_A);
    let c_tx_zero = commit(V_TX, r_tx_zero);
    let v_tilde_zero = encrypt_amount(V_TX, s_x_zero, SIGMA_A);
    let s_a_r_x_zero = ecdh(0, k_aud_r);
    let m_r_zero = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_zero, SIGMA_A);
    let s_a_s_x_zero = ecdh(0, k_aud_s);
    let m_s_zero = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_zero, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        0,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        c_tx_zero.x,
        c_tx_zero.y,
        0,
        0,
        v_tilde_zero,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TX + m_r_zero[0],
        r_tx_zero + m_r_zero[1],
        V_TX + m_s_zero[0],
        V_A_NEW + m_s_zero[1],
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: O6 fails because scalar_mul(r_e+1, H)
    // no longer equals the canonical (R_E_X, R_E_Y). R_e itself is left
    // canonical (so O6 catches the mismatch) but every other r_e-dependent
    // public (C_tx, v_tilde, both auditor channels) is recomputed against
    // r_e+1 so O8 / O9 / O_a3 / O_a4 / O_a7 / O_a8 cannot coincidentally
    // fire.
    let r_e_bad: Field = R_E + 1;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_x_bad = ecdh(r_e_bad, pvk_recipient);
    let r_tx_bad = derive_tx_blind(s_x_bad, SIGMA_A);
    let c_tx_bad = commit(V_TX, r_tx_bad);
    let v_tilde_bad = encrypt_amount(V_TX, s_x_bad, SIGMA_A);
    let s_a_r_x_bad = ecdh(r_e_bad, k_aud_r);
    let m_r_bad = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x_bad, SIGMA_A);
    let s_a_s_x_bad = ecdh(r_e_bad, k_aud_s);
    let m_s_bad = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x_bad, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        r_e_bad,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        c_tx_bad.x,
        c_tx_bad.y,
        R_E_X,
        R_E_Y,
        v_tilde_bad,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TX + m_r_bad[0],
        r_tx_bad + m_r_bad[1],
        V_TX + m_s_bad[0],
        V_A_NEW + m_s_bad[1],
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_r() {
    // (1, 2) is off-curve. The on-curve check on K_aud_r fires before O_a1
    // ever consumes it.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        1,
        2,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_r() {
    // K_aud_r = identity collapses ECDH (S_{a,r} = r_e * O = O), so the
    // on-curve check explicitly rejects the identity for keys.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        0,
        0,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        1,
        2,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        0,
        0,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_r() {
    // O_a3 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R + 1,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_r_tilde_aud_r() {
    // O_a4 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R + 1,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde_aud_s() {
    // O_a7 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S + 1,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_a_tilde_aud_s() {
    // O_a8 fires.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S + 1,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_r() {
    // K_aud_r replaced by a valid but different on-curve key (Y_op). Shared
    // secret changes, recipient-channel masks change, O_a3 fails.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        Y_OP_X,
        Y_OP_Y,
        K_AUD_S_X,
        K_AUD_S_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_k_aud_s() {
    // K_aud_s replaced by a valid but different on-curve key (Y_op). Shared
    // secret changes, owner-channel masks change, O_a7 fails.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TX,
        R_E,
        C_A_X,
        C_A_Y,
        SIGMA_A,
        Y_OP_X,
        Y_OP_Y,
        PVK_RECIPIENT_X,
        PVK_RECIPIENT_Y,
        K_AUD_R_X,
        K_AUD_R_Y,
        Y_OP_X,
        Y_OP_Y,
        C_A_NEW_X,
        C_A_NEW_Y,
        C_TX_X,
        C_TX_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
    );
}
