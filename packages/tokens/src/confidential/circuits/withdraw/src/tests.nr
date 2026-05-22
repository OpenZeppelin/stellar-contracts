use crate::main;
use stellar_confidential_lib::{commit, derive_spend_r, encrypt_balance, H, scalar_mul, vk_from_sk};

// Canonical fixture inputs.
//
// SK / WRAP / SIGMA / V / R reuse the lib's pinned vectors (lib/testdata)
// so that Y, C_spend, vk, r_new, and b_tilde are derivable from -- and
// pinned by -- the lib's `fixtures_match_testdata` test. A is chosen to
// be strictly less than V so the happy path exercises a non-trivial
// `v_new = v - a` while still satisfying W4.
global SK: Field = 0xdead;
global WRAP: Field = 0xbeef;
global SIGMA: Field = 0x01;
global V: Field = 1000;
global R: Field = 42;
global A: Field = 300;
global V_NEW: Field = 700;

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
    0x2fb2d785d71d4c62ce4130675a6c44020c03a236ab21450b6f20282549caadac;
global C_SPEND_NEW_Y: Field =
    0x069a978c07b99e97858a6e94cef5b3d2c49f1df0d2aa5e04dffcefaca9320814;
global B_TILDE: Field =
    0x2b7f92c72c05ae8ecf7207fdaecea32ef748cd4dd43ca9ed6dc6d226865b4283;

#[test]
fn print_fixtures() {
    // One-shot harness: prints C_spend', b_tilde for the chosen (SK, WRAP,
    // SIGMA, V, R, A) tuple. Run with
    // `nargo test --package circuit_withdraw print_fixtures --show-output`
    // and paste the values into the C_SPEND_NEW_* / B_TILDE globals above.
    let vk = vk_from_sk(SK, WRAP);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(V_NEW, r_new);
    let b_tilde = encrypt_balance(V_NEW, vk, SIGMA);
    let cx = c_new.x;
    let cy = c_new.y;
    println(f"C_SPEND_NEW_X = {cx}");
    println(f"C_SPEND_NEW_Y = {cy}");
    println(f"B_TILDE = {b_tilde}");
}

#[test]
fn withdraw_fixtures_match_lib() {
    // Cross-check the hard-coded fixture values against the lib's
    // primitives. If a lib primitive's output changes the fixture
    // assertions break here -- which is exactly when the rest of the
    // withdraw tests would silently start using stale public inputs.
    let vk = vk_from_sk(SK, WRAP);
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
fn matches_fixture() {
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test]
fn full_withdrawal() {
    // Boundary: a = v, so v_new = 0. C_spend' commits to (0, r_new) which
    // is a valid commitment to zero (and is non-identity because r_new is
    // a Poseidon output, vanishingly unlikely to equal 0). W6 binds it.
    let vk = vk_from_sk(SK, WRAP);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new = commit(0, r_new);
    let b_tilde = encrypt_balance(0, vk, SIGMA);
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        V,
        c_new.x,
        c_new.y,
        SIGMA,
        b_tilde,
    );
}

#[test(should_fail)]
fn rejects_under_funded_withdrawal() {
    // a > v: v - a underflows in the field to a value >= 2^128. W4's
    // range check on v_new fires before any downstream constraint, so
    // this is exactly the under-funded-withdraw soundness gap closed.
    let a_too_large: Field = V + 1;
    // C_spend_new is whatever the (necessarily wrong) prover supplies;
    // W6 would also fail, but W4 should fail first. Pick C_spend_new for
    // some valid v_new derivation just to keep the test focused on W4.
    let vk = vk_from_sk(SK, WRAP);
    let r_new = derive_spend_r(vk, SIGMA);
    let c_new_invalid = commit(V - a_too_large, r_new);
    let b_tilde_invalid = encrypt_balance(V - a_too_large, vk, SIGMA);
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        a_too_large,
        c_new_invalid.x,
        c_new_invalid.y,
        SIGMA,
        b_tilde_invalid,
    );
}

#[test(should_fail)]
fn rejects_v_out_of_range() {
    // v claimed by the prover is >= 2^128: W4 rejects on the v check
    // before W3 even looks at C_spend.
    let v_huge: Field = 0x100000000000000000000000000000000; // 2^128
    main(
        SK,
        v_huge,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test(should_fail)]
fn rejects_a_out_of_range() {
    // a >= 2^128: W4's range check on a fails. (The wrapper's a >= 0
    // entrypoint check is out-of-scope for the circuit; here we only
    // close the upper bound.)
    let a_huge: Field = 0x100000000000000000000000000000000;
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        a_huge,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test(should_fail)]
fn rejects_wrong_sk() {
    // sk does not derive the claimed Y: W1 fails.
    main(
        SK + 1,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test(should_fail)]
fn rejects_wrong_balance_opening() {
    // (v, r) supplied don't open C_spend: W3 fails. Use a v that still
    // passes the W4 range check so the failure is squarely W3.
    main(
        SK,
        V + 1,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test(should_fail)]
fn rejects_wrong_wrap() {
    // wrap != WRAP under which Y was derived. vk recomputes under the
    // new wrap, the new vk produces a different r_new (W5) and a
    // different b_tilde (W7). W6 / W7 fire. (W1 still passes because Y
    // is the prover-claimed Y and sk*H is recomputed against it without
    // using wrap.)
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        0xcafe,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}

#[test(should_fail)]
fn rejects_tampered_b_tilde() {
    // b_tilde mutated by +1: W7 fails because encrypt_balance derives a
    // specific scalar that no longer matches.
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE + 1,
    );
}

#[test(should_fail)]
fn rejects_tampered_c_spend_new() {
    // C_spend' supplied with wrong x: W6 fails because the prover's
    // claim no longer matches the in-circuit (v - a)*G + r'*H.
    main(
        SK,
        V,
        R,
        C_SPEND_X,
        C_SPEND_Y,
        Y_X,
        Y_Y,
        WRAP,
        A,
        C_SPEND_NEW_X + 1,
        C_SPEND_NEW_Y,
        SIGMA,
        B_TILDE,
    );
}
