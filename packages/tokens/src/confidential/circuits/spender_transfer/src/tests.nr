use crate::main;
use stellar_confidential_lib::{
    commit, derive_allow_r, derive_transfer_blind, domain, ecdh, encrypt_allowance, encrypt_amount, H,
    scalar_mul, sponge_squeeze_2, sponge_squeeze_3,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Canonical fixture inputs.
//
// SIGMA_A is the stored allowance salt and opens C_a (O3) and nothing else.
// SIGMA_A_NEW is the channel nonce: the recipient ECDH chain (O7 / O9), both
// auditor sponges (O_a2 / O_a6), and the post-transfer allowance derivations
// (O10, O12) all absorb it -- see main.nr "Channel nonce". V_A / V_TRANSFER /
// V_A_NEW give a non-trivial `v_a - v_transfer` so O4 is exercised on a real
// subtraction.
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
global V_TRANSFER: Field = 100;
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
global C_TRANSFER_X: Field = 0x278bb372b27575f08ab88b27d967546c867287e9e64b58a482b4f6433ddf8bdd;
global C_TRANSFER_Y: Field = 0x01021c5c2d8c3d736a8d87555ddd4cef3abc8297657ae139ab0968ff9cfebbfd;
global R_E_X: Field = 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f;
global R_E_Y: Field = 0x07a70cf826350d4f438c7a3c5e8761b0ae6cb63de757f0c96815f4057b9205f4;
global V_TILDE: Field = 0x06656a0ee0a408f201f01030e6a6941a55cb2829549b5f5dd8ed233eb61a7880;
global A_TILDE_NEW: Field =
    0x1d2e286a8d510a7c4164d0142ceea3202a957d49248426435f311a348f681147;
global V_TILDE_AUD_R: Field =
    0x274f7ec87c29f7e31d3240257d563a77373efd3f976bb197f0505fc3eff581dd;
global R_TILDE_AUD_R: Field =
    0x11504bfdaa7e3dfb4b38918d6ae2ba379abfda0a53ebc00fd1b561e805407c96;
global V_TILDE_AUD_S: Field =
    0x08ad26221dfe94a593dae37d3efdbe93a503a3cfe35bd10c3ab60061267c20fc;
// Lane-2 blinding-escrow slot (O_a9): the NEW allowance blinding r_a', the
// one O11 commits C_a' under, escrowed to the owner's auditor on every
// spender transfer. Pad and plaintext both key off SIGMA_A_NEW.
global R_TILDE_AUD_S: Field =
    0x19b2c06c2ec9cacaa9f36876d6b988d186ad82b2f47cff17dbe6e4632d79b4d7;
global A_TILDE_AUD_S: Field =
    0x29499e6f15cad25108f6b3931d8f3cd70cd0c0000ca03921509c5466feb773bb;

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

    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let c_transfer = commit(V_TRANSFER, r_transfer);
    let v_tilde = encrypt_amount(V_TRANSFER, s, SIGMA_A_NEW);

    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(V_A_NEW, r_a_new);
    let a_tilde_new = encrypt_allowance(V_A_NEW, DVK_I, SIGMA_A_NEW);

    let s_a_r = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, SIGMA_A_NEW);
    let v_tilde_aud_r = V_TRANSFER + m_r[0];
    let r_tilde_aud_r = r_transfer + m_r[1];

    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);
    let v_tilde_aud_s = V_TRANSFER + m_s[0];
    let a_tilde_aud_s = V_A_NEW + m_s[1];
    let r_tilde_aud_s = r_a_new + m_s[2];

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
    let c_transfer_x = c_transfer.x;
    let c_transfer_y = c_transfer.y;
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
    println(f"C_TRANSFER_X = {c_transfer_x}");
    println(f"C_TRANSFER_Y = {c_transfer_y}");
    println(f"R_E_X = {rex}");
    println(f"R_E_Y = {rey}");
    println(f"V_TILDE = {v_tilde}");
    println(f"A_TILDE_NEW = {a_tilde_new}");
    println(f"V_TILDE_AUD_R = {v_tilde_aud_r}");
    println(f"R_TILDE_AUD_R = {r_tilde_aud_r}");
    println(f"V_TILDE_AUD_S = {v_tilde_aud_s}");
    println(f"A_TILDE_AUD_S = {a_tilde_aud_s}");
    println(f"R_TILDE_AUD_S = {r_tilde_aud_s}");
}

#[test]
fn spender_transfer_fixtures_match_lib() {
    let y_op = scalar_mul(SK_OP, H);
    let r_a = derive_allow_r(DVK_I, SIGMA_A);
    let c_a = commit(V_A, r_a);
    let pvk_recipient = scalar_mul(VK_RECIPIENT, H);
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let c_transfer = commit(V_TRANSFER, r_transfer);
    let v_tilde = encrypt_amount(V_TRANSFER, s, SIGMA_A_NEW);
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
    assert(c_transfer.x == C_TRANSFER_X);
    assert(c_transfer.y == C_TRANSFER_Y);
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
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let s_a_r = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

    assert(k_aud_r.x == K_AUD_R_X);
    assert(k_aud_r.y == K_AUD_R_Y);
    assert(k_aud_s.x == K_AUD_S_X);
    assert(k_aud_s.y == K_AUD_S_Y);
    assert(r_e_pt.x == R_E_X);
    assert(r_e_pt.y == R_E_Y);
    assert(V_TRANSFER + m_r[0] == V_TILDE_AUD_R);
    assert(r_transfer + m_r[1] == R_TILDE_AUD_R);
    assert(V_TRANSFER + m_s[0] == V_TILDE_AUD_S);
    assert(V_A_NEW + m_s[1] == A_TILDE_AUD_S);
    assert(derive_allow_r(DVK_I, SIGMA_A_NEW) + m_s[2] == R_TILDE_AUD_S);
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
    v_transfer_in: Field,
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
    c_transfer_x_in: Field,
    c_transfer_y_in: Field,
    r_e_x_in: Field,
    r_e_y_in: Field,
    v_tilde_in: Field,
    a_tilde_new_in: Field,
    sigma_a_new_in: Field,
    v_tilde_aud_r_in: Field,
    r_tilde_aud_r_in: Field,
    v_tilde_aud_s_in: Field,
    a_tilde_aud_s_in: Field,
    r_tilde_aud_s_in: Field,
) {
    main(
        sk_op_in,
        dvk_i_in,
        v_a_in,
        r_a_in,
        v_transfer_in,
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
        c_transfer_x_in,
        c_transfer_y_in,
        r_e_x_in,
        r_e_y_in,
        v_tilde_in,
        a_tilde_new_in,
        sigma_a_new_in,
        v_tilde_aud_r_in,
        r_tilde_aud_r_in,
        v_tilde_aud_s_in,
        a_tilde_aud_s_in,
        r_tilde_aud_s_in,
    );
}

fn run_fixture() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test]
fn matches_fixture() {
    run_fixture();
}

#[test]
fn second_transfer_against_same_allowance() {
    // Second happy-path point: same C_a (= same dvk_i, same v_a, same
    // sigma_a) with a different v_transfer. This is the retry / second-attempt
    // shape: the stored sigma_a is forced, so a fresh sigma_a_new is the only
    // thing that can make the pads fresh. Every pad here is keyed to
    // sigma_a_new_2 and r_e_2, so none of the canonical fixture's ciphertexts
    // recur even though C_a, v_a and sigma_a are identical.
    let v_transfer_2: Field = 250;
    let v_a_new_2: Field = V_A - v_transfer_2; // 750
    let sigma_a_new_2: Field = 0x03;
    let r_e_2: Field = 0xfacecafe;

    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };

    let s = ecdh(r_e_2, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, sigma_a_new_2);
    let c_transfer = commit(v_transfer_2, r_transfer);
    let v_tilde_2 = encrypt_amount(v_transfer_2, s, sigma_a_new_2);
    let r_a_new_2 = derive_allow_r(DVK_I, sigma_a_new_2);
    let c_a_new_2 = commit(v_a_new_2, r_a_new_2);
    let a_tilde_new_2 = encrypt_allowance(v_a_new_2, DVK_I, sigma_a_new_2);
    let r_e_2_pt = scalar_mul(r_e_2, H);
    let s_a_r = ecdh(r_e_2, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, sigma_a_new_2);
    let s_a_s = ecdh(r_e_2, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, sigma_a_new_2);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_transfer_2,
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
        c_transfer.x,
        c_transfer.y,
        r_e_2_pt.x,
        r_e_2_pt.y,
        v_tilde_2,
        a_tilde_new_2,
        sigma_a_new_2,
        v_transfer_2 + m_r[0],
        r_transfer + m_r[1],
        v_transfer_2 + m_s[0],
        v_a_new_2 + m_s[1],
        r_a_new_2 + m_s[2],
    );
}

#[test]
fn full_allowance_transfer() {
    // Boundary: v_transfer = v_a, so v_a_new = 0. C_a' commits to (0, r_a_new'),
    // which is a valid commitment to zero. a_tilde' and a_tilde_aud_s both
    // bind to 0 (plus mask) so the owner's auditor sees the post-transfer
    // allowance reach zero. The delegation entry isn't deleted (that's
    // revoke_spender's job, Section 7.9) -- it now escrows zero value.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let c_transfer = commit(V_A, r_transfer);
    let v_tilde = encrypt_amount(V_A, s, SIGMA_A_NEW);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(0, r_a_new);
    let a_tilde_new = encrypt_allowance(0, DVK_I, SIGMA_A_NEW);
    let s_a_r = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new,
        SIGMA_A_NEW,
        V_A + m_r[0],
        r_transfer + m_r[1],
        V_A + m_s[0],
        0 + m_s[1],
        r_a_new + m_s[2],
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_wrong_allowance_opening() {
    // O2 fires: commit(V_A + 1, R_A) != C_a. v_a_new cascades into O11 / O12
    // / O_a8; recompute c_a_new, a_tilde_new, and a_tilde_aud_s against the
    // tampered v_a_new so only O2 fires.
    let v_a_bad: Field = V_A + 1;
    let v_a_new_bad: Field = v_a_bad - V_TRANSFER;
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new_bad = commit(v_a_new_bad, r_a_new);
    let a_tilde_new_bad = encrypt_allowance(v_a_new_bad, DVK_I, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        v_a_bad,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        a_tilde_new_bad,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_a_new_bad + m_s[1],
        r_a_new + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_v_a_new_out_of_range() {
    // v_transfer > v_a: v_a - v_transfer underflows in the field to a value >= 2^128.
    // O4's `v_a_new` range check fires. Every downstream public is
    // recomputed against v_transfer_too_large so the single firing constraint is
    // O4's v_a_new range check (the v_a and v_transfer individual sub-checks pass:
    // V_A is in range and v_transfer_too_large = V_A + 1 = 1001 is also in range).
    let v_transfer_too_large: Field = V_A + 1;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let c_transfer = commit(v_transfer_too_large, r_transfer);
    let v_tilde = encrypt_amount(v_transfer_too_large, s, SIGMA_A_NEW);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new_invalid = commit(V_A - v_transfer_too_large, r_a_new);
    let a_tilde_new_invalid = encrypt_allowance(V_A - v_transfer_too_large, DVK_I, SIGMA_A_NEW);
    let s_a_r = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_transfer_too_large,
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new_invalid,
        SIGMA_A_NEW,
        v_transfer_too_large + m_r[0],
        r_transfer + m_r[1],
        v_transfer_too_large + m_s[0],
        (V_A - v_transfer_too_large) + m_s[1],
        r_a_new + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_v_a_out_of_range() {
    // 2^127 is exactly the boundary O4 rules out (Section 2.6 / 3.4 -- SEP-41
    // i128 non-negative range is [0, 2^127), so 2^127 is the smallest value
    // that must fail). With v_a_huge - V_TRANSFER = 2^127 - 100 still in [0, 2^127),
    // the v_a_new range check passes; only the `v_a.assert_max_bit_size<127>()`
    // sub-check of O4 fires. C_a, c_a_new, and a_tilde_aud_s are all
    // recomputed against v_a_huge / v_a_new_huge so O2 / O11 / O12 / O_a8
    // cannot coincidentally fire. R_A is independent of v_a so it stays
    // canonical.
    let v_a_huge: Field = 0x80000000000000000000000000000000;
    let v_a_new_huge: Field = v_a_huge - V_TRANSFER;
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let c_a = commit(v_a_huge, R_A);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(v_a_new_huge, r_a_new);
    let a_tilde_new = encrypt_allowance(v_a_new_huge, DVK_I, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        v_a_huge,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        a_tilde_new,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        v_a_new_huge + m_s[1],
        r_a_new + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_v_transfer_out_of_range() {
    // v_transfer = 2^127: O4's range check on v_transfer fires. Every downstream public
    // that depends on v_transfer (C_transfer, v_tilde, v_tilde_aud_r, v_tilde_aud_s) and
    // on v_a_new (c_a_new, a_tilde_new, a_tilde_aud_s) is recomputed so the
    // only constraints that fail are O4's range sub-checks.
    let v_transfer_huge: Field = 0x80000000000000000000000000000000;
    let v_a_new_huge: Field = V_A - v_transfer_huge;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A_NEW);
    let c_transfer = commit(v_transfer_huge, r_transfer);
    let v_tilde = encrypt_amount(v_transfer_huge, s, SIGMA_A_NEW);
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A_NEW);
    let c_a_new = commit(v_a_new_huge, r_a_new);
    let a_tilde_new = encrypt_allowance(v_a_new_huge, DVK_I, SIGMA_A_NEW);
    let s_a_r = ecdh(R_E, k_aud_r);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, SIGMA_A_NEW);
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        v_transfer_huge,
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new,
        SIGMA_A_NEW,
        v_transfer_huge + m_r[0],
        r_transfer + m_r[1],
        v_transfer_huge + m_s[0],
        v_a_new_huge + m_s[1],
        r_a_new + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_wrong_recipient_pvk() {
    // PVK_recipient replaced by Y_op (a valid on-curve point that isn't the
    // registered recipient's PVK). S = r_e * PVK_recipient changes, so r_transfer
    // and v_tilde drift and O8 fails (C_transfer no longer matches commit(v_transfer,
    // r_transfer_new)).
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_poisoned_c_transfer() {
    // Anti-poisoning (Section 5.4): a prover that commits C_transfer with arbitrary
    // blinding -- here r_transfer + 1 -- desynchronizes the recipient's accumulated
    // blinding. O7 + O8 together close this attack: r_transfer is uniquely
    // determined by Poseidon2(delta_transfer_blind, s, sigma_a'), so any C_transfer not
    // built from that exact r_transfer fails O8.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer_poisoned = derive_transfer_blind(s, SIGMA_A_NEW) + 1;
    let c_transfer_poisoned = commit(V_TRANSFER, r_transfer_poisoned);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        c_transfer_poisoned.x,
        c_transfer_poisoned.y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_tampered_v_tilde() {
    // O9 fires: v_tilde + 1 no longer matches v_transfer + Poseidon2(TRANSFER_AMOUNT, ...).
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE + 1,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW + 1,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_sigma_a_new_equal_to_sigma_a() {
    // O14: the circuit rejects `sigma_a' == sigma_a`, which would key every
    // pad to the salt the previous transfer already used. With
    // `sigma_a' = sigma_a`:
    //   r_a_new = derive_allow_r(dvk_i, sigma_a_new) = r_a
    //   c_a_new = (V_A - V_TRANSFER) * G + r_a * H
    //   a_tilde_new = encrypt_allowance(V_A - V_TRANSFER, dvk_i, sigma_a_new)
    // and every channel pad absorbs SIGMA_A. All downstream publics are
    // recomputed consistently so the only firing constraint is
    // `assert(sigma_a_new != sigma_a)`.
    let r_a_new = derive_allow_r(DVK_I, SIGMA_A);
    let c_a_new = commit(V_A_NEW, r_a_new);
    let a_tilde_new = encrypt_allowance(V_A_NEW, DVK_I, SIGMA_A);

    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let r_transfer = derive_transfer_blind(s, SIGMA_A);
    let c_transfer = commit(V_TRANSFER, r_transfer);
    let v_tilde = encrypt_amount(V_TRANSFER, s, SIGMA_A);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, ecdh(R_E, k_aud_r), SIGMA_A);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, ecdh(R_E, k_aud_s), SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        c_transfer.x,
        c_transfer.y,
        R_E_X,
        R_E_Y,
        v_tilde,
        a_tilde_new,
        SIGMA_A,
        V_TRANSFER + m_r[0],
        r_transfer + m_r[1],
        V_TRANSFER + m_s[0],
        V_A_NEW + m_s[1],
        r_a_new + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_r_e_zero() {
    // O13 fires: r_e = 0 would force R_e = O (identity) and collapse every
    // shared point in this transfer (S, S_{a,r}, S_{a,s} all become O), so
    // each ecdh(0, .) returns the ECDH-domain hash of the identity point:
    // s_zero, s_a_r_zero, and s_a_s_zero are identity-derived constants,
    // making every mask a knowable constant function of sigma_a'. Every
    // r_e-dependent public (R_e = (0, 0) identity-encoding, C_transfer /
    // v_tilde derived against s_zero, and both auditor channels' ciphertexts
    // derived against s_a_r_zero / s_a_s_zero) is recomputed so the
    // `assert(r_e != 0)` check is the only constraint that can catch this --
    // removing O13 would let the proof verify.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_zero = ecdh(0, pvk_recipient);
    let r_transfer_zero = derive_transfer_blind(s_zero, SIGMA_A_NEW);
    let c_transfer_zero = commit(V_TRANSFER, r_transfer_zero);
    let v_tilde_zero = encrypt_amount(V_TRANSFER, s_zero, SIGMA_A_NEW);
    let s_a_r_zero = ecdh(0, k_aud_r);
    let m_r_zero = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_zero, SIGMA_A_NEW);
    let s_a_s_zero = ecdh(0, k_aud_s);
    let m_s_zero = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s_zero, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        c_transfer_zero.x,
        c_transfer_zero.y,
        0,
        0,
        v_tilde_zero,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TRANSFER + m_r_zero[0],
        r_transfer_zero + m_r_zero[1],
        V_TRANSFER + m_s_zero[0],
        V_A_NEW + m_s_zero[1],
        derive_allow_r(DVK_I, SIGMA_A_NEW) + m_s_zero[2],
    );
}

#[test(should_fail)]
fn rejects_wrong_r_e() {
    // r_e mismatches the public R_e: O6 fails because scalar_mul(r_e+1, H)
    // no longer equals the canonical (R_E_X, R_E_Y). R_e itself is left
    // canonical (so O6 catches the mismatch) but every other r_e-dependent
    // public (C_transfer, v_tilde, both auditor channels) is recomputed against
    // r_e+1 so O8 / O9 / O_a3 / O_a4 / O_a7 / O_a8 cannot coincidentally
    // fire.
    let r_e_bad: Field = R_E + 1;
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_bad = ecdh(r_e_bad, pvk_recipient);
    let r_transfer_bad = derive_transfer_blind(s_bad, SIGMA_A_NEW);
    let c_transfer_bad = commit(V_TRANSFER, r_transfer_bad);
    let v_tilde_bad = encrypt_amount(V_TRANSFER, s_bad, SIGMA_A_NEW);
    let s_a_r_bad = ecdh(r_e_bad, k_aud_r);
    let m_r_bad = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_bad, SIGMA_A_NEW);
    let s_a_s_bad = ecdh(r_e_bad, k_aud_s);
    let m_s_bad = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s_bad, SIGMA_A_NEW);

    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        c_transfer_bad.x,
        c_transfer_bad.y,
        R_E_X,
        R_E_Y,
        v_tilde_bad,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TRANSFER + m_r_bad[0],
        r_transfer_bad + m_r_bad[1],
        V_TRANSFER + m_s_bad[0],
        V_A_NEW + m_s_bad[1],
        derive_allow_r(DVK_I, SIGMA_A_NEW) + m_s_bad[2],
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_off_curve_k_aud_s() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}

#[test(should_fail)]
fn rejects_identity_k_aud_s() {
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R + 1,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R + 1,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S + 1,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S + 1,
        R_TILDE_AUD_S,
    );
}

// Salt and ephemeral reuse (characterization)
// -------------------------------------------
// The only freshness this circuit enforces is `sigma_a' != sigma_a`, and only
// against the immediate predecessor; `r_e` is unchecked. The tests below pin
// what happens under reuse rather than asserting it is prevented.

#[test]
fn alternating_sigma_a_reproduces_an_earlier_allowance_blinding() {
    // A -> B -> A is accepted: the salt-rotation check only rules out the
    // state the transfer consumes. Since r_a is a pure function of
    // (dvk_i, sigma_a), returning to salt A reproduces salt A's blinding
    // exactly -- R_A, the blinding of the very first allowance commitment. The
    // consequence is linkability, not unsoundness: if the value also
    // coincided, C_a would be byte-identical to a commitment already on-chain.
    // Here the value differs, so only the blinding repeats.
    let v_transfer_2: Field = 400;
    let v_a_new_2: Field = V_A_NEW - v_transfer_2; // 500
    let r_e_2: Field = R_E + 3;

    // Back to SIGMA_A after one hop through SIGMA_A_NEW; the pads follow the
    // written salt, so they key off SIGMA_A too.
    let r_a_new_2 = derive_allow_r(DVK_I, SIGMA_A);
    assert(r_a_new_2 == R_A);

    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(r_e_2, pvk_recipient);
    let r_transfer_2 = derive_transfer_blind(s, SIGMA_A);
    let c_transfer_2 = commit(v_transfer_2, r_transfer_2);
    let v_tilde_2 = encrypt_amount(v_transfer_2, s, SIGMA_A);
    let c_a_new_2 = commit(v_a_new_2, r_a_new_2);
    let a_tilde_new_2 = encrypt_allowance(v_a_new_2, DVK_I, SIGMA_A);
    let r_e_2_pt = scalar_mul(r_e_2, H);
    let s_a_r_2 = ecdh(r_e_2, k_aud_r);
    let m_r_2 = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_2, SIGMA_A);
    let s_a_s_2 = ecdh(r_e_2, k_aud_s);
    let m_s_2 = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s_2, SIGMA_A);

    run_main(
        SK_OP,
        DVK_I,
        V_A_NEW,
        derive_allow_r(DVK_I, SIGMA_A_NEW),
        v_transfer_2,
        r_e_2,
        C_A_NEW_X,
        C_A_NEW_Y,
        SIGMA_A_NEW,
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
        c_transfer_2.x,
        c_transfer_2.y,
        r_e_2_pt.x,
        r_e_2_pt.y,
        v_tilde_2,
        a_tilde_new_2,
        SIGMA_A,
        v_transfer_2 + m_r_2[0],
        r_transfer_2 + m_r_2[1],
        v_transfer_2 + m_s_2[0],
        v_a_new_2 + m_s_2[1],
        r_a_new_2 + m_s_2[2],
    );
}

#[test]
fn repeated_r_e_and_sigma_a_new_repeat_every_channel_pad() {
    // Reusing (r_e, sigma_a') repeats s, s_{a,r}, and s_{a,s}, and with them
    // every mask on all three channels. Re-encrypting the same plaintexts
    // under them republishes byte-identical ciphertexts -- no differential.
    let pvk_recipient =
        EmbeddedCurvePoint { x: PVK_RECIPIENT_X, y: PVK_RECIPIENT_Y, is_infinite: false };
    let k_aud_r = EmbeddedCurvePoint { x: K_AUD_R_X, y: K_AUD_R_Y, is_infinite: false };
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s = ecdh(R_E, pvk_recipient);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, ecdh(R_E, k_aud_r), SIGMA_A_NEW);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, ecdh(R_E, k_aud_s), SIGMA_A_NEW);
    assert(encrypt_amount(V_TRANSFER, s, SIGMA_A_NEW) == V_TILDE);
    assert(V_TRANSFER + m_r[0] == V_TILDE_AUD_R);
    assert(V_TRANSFER + m_s[0] == V_TILDE_AUD_S);
    assert(V_A_NEW + m_s[1] == A_TILDE_AUD_S);
    assert(derive_allow_r(DVK_I, SIGMA_A_NEW) + m_s[2] == R_TILDE_AUD_S);

    // Change a plaintext under the repeated pad and the difference survives in
    // the clear. Two transfers of different sizes under one (r_e, sigma_a')
    // publish v_tilde values whose difference is the amount delta. The circuit
    // cannot catch this -- a reused r_e is a well-formed witness and O14 sees
    // only the adjacent salt -- so it is the honest client's obligation
    // (Section 5.3, Section 9.6, SDK.md Section 10.4).
    let v_transfer_2: Field = V_TRANSFER + 40;
    assert((v_transfer_2 + m_s[0]) - V_TILDE_AUD_S == 40);
}

#[test(should_fail)]
fn rejects_escrow_of_the_consumed_allowance_blinding() {
    // The prover escrows R_A -- the blinding of the allowance this transfer
    // CONSUMES -- instead of O10's r_a', the blinding of the allowance it
    // writes. Correctly padded under the canonical lane-2 mask, so only O_a9
    // can catch it, and it does.
    //
    // The distinction matters because the auditor's job is to track live
    // state: a_tilde_aud_s (O_a8) already carries the post-transfer value, so
    // pairing it with the pre-transfer blinding yields an opening of nothing
    // on-chain. The auditor already knew the consumed opening -- it decrypted
    // it from the previous operation's event.
    let k_aud_s = EmbeddedCurvePoint { x: K_AUD_S_X, y: K_AUD_S_Y, is_infinite: false };
    let s_a_s = ecdh(R_E, k_aud_s);
    let m_s = sponge_squeeze_3(domain::AUDITOR_SENDER, s_a_s, SIGMA_A_NEW);
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_A + m_s[2],
    );
}

#[test(should_fail)]
fn rejects_tampered_r_tilde_aud_s() {
    // r_tilde_aud_s mutated by +1: O_a9 fails because the lane-2 escrow is
    // pinned to O10's r_a', which the prover's claim no longer matches.
    run_main(
        SK_OP,
        DVK_I,
        V_A,
        R_A,
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S + 1,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
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
        V_TRANSFER,
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
        C_TRANSFER_X,
        C_TRANSFER_Y,
        R_E_X,
        R_E_Y,
        V_TILDE,
        A_TILDE_NEW,
        SIGMA_A_NEW,
        V_TILDE_AUD_R,
        R_TILDE_AUD_R,
        V_TILDE_AUD_S,
        A_TILDE_AUD_S,
        R_TILDE_AUD_S,
    );
}
