use crate::main;
use stellar_confidential_lib::{commit, scalar_mul, H};

// Canonical fixture inputs.
//
// V_S / R_S reuse the lib's pinned `commit` vector (lib/testdata), so C_SPEND
// is derivable from -- and pinned by -- the lib's `fixtures_match_testdata`
// test. V_R / R_R and ALPHA are local to this circuit. ALPHA = 700 exceeds
// V_R alone, so the happy path exercises a seize that necessarily draws on
// both commitments rather than one. K_AUD_SCALAR is the same placeholder
// auditor secret the withdraw fixtures use, so K_AUD is the withdraw
// `K_AUD_S` point.
global K_AUD_SCALAR: Field = 0xc0ffee;
global V_S: Field = 1000;
global R_S: Field = 42;
global V_R: Field = 500;
global R_R: Field = 77;
global ALPHA: Field = 700;

global ADDR_F: Field = 0xbeef;
global ACCT_F: Field = 0xfeed;
global DEST_F: Field = 0xcafe;
global NONCE: Field = 3;

// C_spend = commit(1000, 42) (= lib `commit` fixture).
global C_SPEND_X: Field = 0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05;
global C_SPEND_Y: Field = 0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e;

// C_receive = commit(500, 77). Pinned by `clawback_fixtures_match_lib`.
global C_RECEIVE_X: Field = 0x13399238a890131dbbe17b7e1d4bf333057906c812fc4c718b2c6efad3e97a08;
global C_RECEIVE_Y: Field = 0x2f5dc4cdf9796114e546848c941e7f7fc09ea5dbf2542ea242005dccffb6b7e8;

// K_aud = K_AUD_SCALAR * H. Pinned by `clawback_fixtures_match_lib`.
global K_AUD_X: Field = 0x22502c7b20b64aeaaaa4d4fa5f2f8600f9734e828f7284a8d1431da36b33803a;
global K_AUD_Y: Field = 0x0419089353d24334f03f4a1c9c9b19282e32f6879c5c69b9cdffba6a13a7c5ed;

#[test]
fn print_fixtures() {
    // One-shot harness: prints C_receive and K_aud for the chosen fixture
    // tuple. Run with
    // `nargo test --package circuit_clawback print_fixtures --show-output`
    // and paste the values into the globals above.
    let c_receive = commit(V_R, R_R);
    let rx = c_receive.x;
    let ry = c_receive.y;
    println(f"C_RECEIVE_X = {rx}");
    println(f"C_RECEIVE_Y = {ry}");
    let k_aud = scalar_mul(K_AUD_SCALAR, H);
    let kx = k_aud.x;
    let ky = k_aud.y;
    println(f"K_AUD_X = {kx}");
    println(f"K_AUD_Y = {ky}");
}

#[test]
fn clawback_fixtures_match_lib() {
    // Pins the fixtures to the lib primitives, so drift is reported here
    // rather than as a less informative failure deep inside `main`.
    let c_spend = commit(V_S, R_S);
    let c_receive = commit(V_R, R_R);
    let k_aud = scalar_mul(K_AUD_SCALAR, H);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
    assert(c_receive.x == C_RECEIVE_X);
    assert(c_receive.y == C_RECEIVE_Y);
    assert(k_aud.x == K_AUD_X);
    assert(k_aud.y == K_AUD_Y);
}

#[test]
fn matches_fixture() {
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test]
fn empty_receiving_side_is_the_identity() {
    // C_receive = O is the common case: the target has nothing pending. The
    // circuit must open it as (0, 0) with no special case, and `commit(0, 0)`
    // must encode as x = y = 0.
    let identity = commit(0, 0);
    assert(identity.x == 0);
    assert(identity.y == 0);

    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        0,
        0,
        C_SPEND_X,
        C_SPEND_Y,
        0,
        0,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test]
fn public_openings_verify_under_the_auditor_key() {
    // An account whose history is register + deposit has C_spend = O and
    // C_receive = a * G, both opened with zero blinding. The auditor can
    // still seize it: CB1 is the only constraint that needs a secret.
    let c_receive = commit(100, 0);
    main(
        K_AUD_SCALAR,
        0,
        0,
        100,
        0,
        0,
        0,
        c_receive.x,
        c_receive.y,
        K_AUD_X,
        K_AUD_Y,
        100,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test]
fn seize_of_the_entire_claim_is_admissible() {
    // alpha == v_s + v_r leaves the account at zero, which is in range.
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        V_S + V_R,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test]
fn none_destination_sentinel_verifies() {
    // The contract passes the zero field for `destination: None`. Nothing in
    // the circuit distinguishes it -- the point of the test is that the
    // sentinel is a legal public input, not a rejected one.
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        0,
        NONCE,
    );
}

#[test(should_fail)]
fn public_openings_without_the_auditor_key_are_rejected() {
    // The same register + deposit account as
    // `public_openings_verify_under_the_auditor_key`, proved by a party that
    // reconstructed the public openings but does not hold k_aud. This is the
    // clawback signer acting alone; CB1 is what rejects it.
    let c_receive = commit(100, 0);
    main(
        0xbad,
        0,
        0,
        100,
        0,
        0,
        0,
        c_receive.x,
        c_receive.y,
        K_AUD_X,
        K_AUD_Y,
        100,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test(should_fail)]
fn wrong_auditor_key_is_rejected() {
    // A prover holding both openings but a different auditor secret fails
    // CB1: the witness is bound to the auditor the account registered under.
    main(
        K_AUD_SCALAR + 1,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test(should_fail)]
fn over_seize_is_rejected() {
    // alpha > v_s + v_r makes v_s + v_r - alpha wrap mod F_r, landing far
    // above 2^127. This is CB4's soundness-critical case: without the sum
    // check the seize would produce a commitment the owner can open but can
    // never spend from again.
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        V_S + V_R + 1,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test(should_fail)]
fn out_of_range_alpha_is_rejected() {
    // alpha >= 2^127 fails CB4's own range check before the sum is formed.
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        0x80000000000000000000000000000000,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test(should_fail)]
fn wrong_spendable_opening_is_rejected() {
    // A witness that does not open the on-chain C_spend fails CB2. This is
    // what keeps the seize bounded by what the account actually holds.
    main(
        K_AUD_SCALAR,
        V_S + 1,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}

#[test(should_fail)]
fn wrong_receiving_opening_is_rejected() {
    // Same for CB3: an inflated v_r would otherwise let a prover claim a
    // larger seizable total than the account holds.
    main(
        K_AUD_SCALAR,
        V_S,
        R_S,
        V_R + 1,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        K_AUD_X,
        K_AUD_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
        NONCE,
    );
}
