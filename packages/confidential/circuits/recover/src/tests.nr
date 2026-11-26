use crate::main;
use stellar_confidential_lib::{
    commit, derive_transfer_blind, domain, ecdh, encrypt_amount, H, scalar_mul, sponge_squeeze_2,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

// Fixture inputs.
//
// V_S / R_S / V_R / R_R are the clawback fixture openings, so C_spend and
// C_receive are the points pinned there. VK_B, K_AUD_R_SCALAR, R_E and SIGMA
// are the transfer fixture scalars, so PVK_B, K_aud_r and R_e are the points
// pinned there. Every other public input is derived below from the lib
// primitives, which `fixtures_match_testdata` pins.
global V_S: Field = 1000;
global R_S: Field = 42;
global V_R: Field = 500;
global R_R: Field = 77;
global VK_B: Field = 0xfeed;
global K_AUD_R_SCALAR: Field = 0xc0ffee01;
global R_E: Field = 0xfeedface;
global SIGMA: Field = 0x01;
global ADDR_F: Field = 0xbeef;

struct Inputs {
    v_s: Field,
    r_s: Field,
    v_r: Field,
    r_r: Field,
    r_e: Field,
    c_spend: EmbeddedCurvePoint,
    c_receive: EmbeddedCurvePoint,
    pvk_b: EmbeddedCurvePoint,
    k_aud_r: EmbeddedCurvePoint,
    c_transfer: EmbeddedCurvePoint,
    r_e_point: EmbeddedCurvePoint,
    v_tilde: Field,
    v_tilde_aud_r: Field,
    r_tilde_aud_r: Field,
}

// Builds the honest public inputs for the given source openings.
fn honest(v_s: Field, r_s: Field, v_r: Field, r_r: Field) -> Inputs {
    let pvk_b = scalar_mul(VK_B, H);
    let k_aud_r = scalar_mul(K_AUD_R_SCALAR, H);
    let v = v_s + v_r;
    let s = ecdh(R_E, pvk_b);
    let r_transfer = derive_transfer_blind(s, SIGMA);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, ecdh(R_E, k_aud_r), SIGMA);
    Inputs {
        v_s,
        r_s,
        v_r,
        r_r,
        r_e: R_E,
        c_spend: commit(v_s, r_s),
        c_receive: commit(v_r, r_r),
        pvk_b,
        k_aud_r,
        c_transfer: commit(v, r_transfer),
        r_e_point: scalar_mul(R_E, H),
        v_tilde: encrypt_amount(v, s, SIGMA),
        v_tilde_aud_r: v + m_r[0],
        r_tilde_aud_r: r_transfer + m_r[1],
    }
}

fn run(i: Inputs) {
    main(
        i.v_s,
        i.r_s,
        i.v_r,
        i.r_r,
        i.r_e,
        i.c_spend.x,
        i.c_spend.y,
        i.c_receive.x,
        i.c_receive.y,
        i.pvk_b.x,
        i.pvk_b.y,
        ADDR_F,
        i.k_aud_r.x,
        i.k_aud_r.y,
        i.c_transfer.x,
        i.c_transfer.y,
        i.r_e_point.x,
        i.r_e_point.y,
        i.v_tilde,
        SIGMA,
        i.v_tilde_aud_r,
        i.r_tilde_aud_r,
    );
}

#[test]
fn matches_fixture() {
    run(honest(V_S, R_S, V_R, R_R));
}

#[test]
fn recipient_opens_the_credit_as_an_incoming_transfer() {
    // The recipient's wallet applies the incoming-transfer rule of
    // docs/protocol/wallet-state.md#update-rules to (R_e, v_tilde, sigma) and
    // must recover the opening of C_transfer.
    let i = honest(V_S, R_S, V_R, R_R);
    let s = ecdh(VK_B, i.r_e_point);
    let v = i.v_tilde - encrypt_amount(0, s, SIGMA);
    let r = derive_transfer_blind(s, SIGMA);
    assert(v == V_S + V_R);
    let c = commit(v, r);
    assert(c.x == i.c_transfer.x);
    assert(c.y == i.c_transfer.y);
}

#[test]
fn empty_receiving_side_is_the_identity() {
    let i = honest(V_S, R_S, 0, 0);
    assert(i.c_receive.x == 0);
    assert(i.c_receive.y == 0);
    run(i);
}

#[test]
fn public_openings_verify() {
    // A source whose history is register + deposit opens with zero blinding.
    run(honest(0, 0, 100, 0));
}

#[test(should_fail)]
fn wrong_spendable_opening_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.v_s = V_S + 1;
    run(i);
}

#[test(should_fail)]
fn wrong_receiving_opening_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.r_r = R_R + 1;
    run(i);
}

#[test(should_fail)]
fn inflated_credit_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    let s = ecdh(R_E, i.pvk_b);
    i.c_transfer = commit(V_S + V_R + 1, derive_transfer_blind(s, SIGMA));
    run(i);
}

#[test(should_fail)]
fn poisoned_credit_is_rejected() {
    // Right value, blinding the recipient cannot derive.
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.c_transfer = commit(V_S + V_R, 7);
    run(i);
}

#[test(should_fail)]
fn out_of_range_total_is_rejected() {
    // Each side is in range; the sum is 2^127.
    let half = 0x40000000000000000000000000000000;
    run(honest(half, R_S, half, R_R));
}

#[test(should_fail)]
fn tampered_v_tilde_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.v_tilde += 1;
    run(i);
}

#[test(should_fail)]
fn r_e_zero_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.r_e = 0;
    run(i);
}

#[test(should_fail)]
fn wrong_r_e_point_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.r_e_point = scalar_mul(R_E + 1, H);
    run(i);
}

#[test(should_fail)]
fn wrong_recipient_key_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.pvk_b = scalar_mul(VK_B + 1, H);
    run(i);
}

#[test(should_fail)]
fn identity_auditor_key_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.k_aud_r = EmbeddedCurvePoint { x: 0, y: 0, is_infinite: false };
    run(i);
}

#[test(should_fail)]
fn wrong_auditor_key_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.k_aud_r = scalar_mul(K_AUD_R_SCALAR + 1, H);
    run(i);
}

#[test(should_fail)]
fn tampered_v_tilde_aud_r_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.v_tilde_aud_r += 1;
    run(i);
}

#[test(should_fail)]
fn tampered_r_tilde_aud_r_is_rejected() {
    let mut i = honest(V_S, R_S, V_R, R_R);
    i.r_tilde_aud_r += 1;
    run(i);
}
