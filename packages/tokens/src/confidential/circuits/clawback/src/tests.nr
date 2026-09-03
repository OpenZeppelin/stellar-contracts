use crate::main;
use stellar_confidential_lib::commit;

// Canonical fixture inputs.
//
// V_S / R_S reuse the lib's pinned `commit` vector (lib/testdata), so C_SPEND
// is derivable from -- and pinned by -- the lib's `fixtures_match_testdata`
// test. V_R / R_R and ALPHA are local to this circuit. ALPHA = 700 exceeds
// V_R alone, so the happy path exercises a seize that necessarily draws on
// both commitments rather than one.
global V_S: Field = 1000;
global R_S: Field = 42;
global V_R: Field = 500;
global R_R: Field = 77;
global ALPHA: Field = 700;

global ADDR_F: Field = 0xbeef;
global ACCT_F: Field = 0xfeed;
global DEST_F: Field = 0xcafe;

// C_spend = commit(1000, 42) (= lib `commit` fixture).
global C_SPEND_X: Field = 0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05;
global C_SPEND_Y: Field = 0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e;

// C_receive = commit(500, 77). Pinned by `clawback_fixtures_match_lib`.
global C_RECEIVE_X: Field = 0x13399238a890131dbbe17b7e1d4bf333057906c812fc4c718b2c6efad3e97a08;
global C_RECEIVE_Y: Field = 0x2f5dc4cdf9796114e546848c941e7f7fc09ea5dbf2542ea242005dccffb6b7e8;

#[test]
fn print_fixtures() {
    // One-shot harness: prints C_receive for the chosen fixture tuple. Run
    // with `nargo test --package circuit_clawback print_fixtures --show-output`
    // and paste the values into the globals above.
    let c_receive = commit(V_R, R_R);
    let rx = c_receive.x;
    let ry = c_receive.y;
    println(f"C_RECEIVE_X = {rx}");
    println(f"C_RECEIVE_Y = {ry}");
}

#[test]
fn clawback_fixtures_match_lib() {
    // Pins the fixtures to the lib primitives, so drift is reported here
    // rather than as a less informative failure deep inside `main`.
    let c_spend = commit(V_S, R_S);
    let c_receive = commit(V_R, R_R);
    assert(c_spend.x == C_SPEND_X);
    assert(c_spend.y == C_SPEND_Y);
    assert(c_receive.x == C_RECEIVE_X);
    assert(c_receive.y == C_RECEIVE_Y);
}

#[test]
fn matches_fixture() {
    main(
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
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

    main(V_S, R_S, 0, 0, C_SPEND_X, C_SPEND_Y, 0, 0, ALPHA, ADDR_F, ACCT_F, DEST_F);
}

#[test]
fn seize_of_the_entire_claim_is_admissible() {
    // alpha == v_s + v_r leaves the account at zero, which is in range.
    main(
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        V_S + V_R,
        ADDR_F,
        ACCT_F,
        DEST_F,
    );
}

#[test]
fn none_destination_sentinel_verifies() {
    // The contract passes the zero field for `destination: None`. Nothing in
    // the circuit distinguishes it -- the point of the test is that the
    // sentinel is a legal public input, not a rejected one.
    main(
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        0,
    );
}

#[test(should_fail)]
fn over_seize_is_rejected() {
    // alpha > v_s + v_r makes v_s + v_r - alpha wrap mod F_r, landing far
    // above 2^127. This is CB3's soundness-critical case: without the sum
    // check the seize would produce a commitment the owner can open but can
    // never spend from again.
    main(
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        V_S + V_R + 1,
        ADDR_F,
        ACCT_F,
        DEST_F,
    );
}

#[test(should_fail)]
fn out_of_range_alpha_is_rejected() {
    // alpha >= 2^127 fails CB3's own range check before the sum is formed.
    main(
        V_S,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        0x80000000000000000000000000000000,
        ADDR_F,
        ACCT_F,
        DEST_F,
    );
}

#[test(should_fail)]
fn wrong_spendable_opening_is_rejected() {
    // A witness that does not open the on-chain C_spend fails CB1. This is
    // what stops the compliance admin -- which holds no blinding -- from
    // producing a witness of its own.
    main(
        V_S + 1,
        R_S,
        V_R,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
    );
}

#[test(should_fail)]
fn wrong_receiving_opening_is_rejected() {
    // Same for CB2: an inflated v_r would otherwise let a prover claim a
    // larger seizable total than the account holds.
    main(
        V_S,
        R_S,
        V_R + 1,
        R_R,
        C_SPEND_X,
        C_SPEND_Y,
        C_RECEIVE_X,
        C_RECEIVE_Y,
        ALPHA,
        ADDR_F,
        ACCT_F,
        DEST_F,
    );
}
