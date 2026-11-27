use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_transfer_blind, domain, ecdh, encrypt_amount,
    scalar_mul, sponge_squeeze_2,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// Recover circuit -- docs/compliance.md#recovery-circuit.
//
// Proves that the commitment credited to the recipient holds exactly the
// source's committed total, re-blinded so that the recipient opens it as it
// opens an incoming transfer. The amount stays private. The proof authorizes
// nothing: it is the recipient's guarantee against poisoning
// (docs/protocol/keys-and-commitments.md#anti-poisoning-constraint).
//
// Constraints
// -----------
// Source openings
//   RV1 C_spend    = v_s * G + r_s * H                      Opening of the
//                                                           source's spendable
//                                                           commitment.
//   RV2 C_receive  = v_r * G + r_r * H                      Opening of the
//                                                           source's receiving
//                                                           commitment.
//   RV3 v_s, v_r, v_s + v_r in [0, 2^127)                   Range validity of
//                                                           the moved total
//                                                           (docs/protocol/primitives.md#integer-embedding-and-range-proofs).
// Recipient ECDH + transfer commitment (T5-T9, T13 of the transfer circuit)
//   RV4 R_e        = r_e * H                                Ephemeral public
//                                                           key.
//   RV5 s          = ecdh(r_e, PVK_B)                       Recipient ECDH
//                                                           shared scalar.
//   RV6 r_transfer = Poseidon2(delta_transfer_blind, s, sigma)
//                                                           Anti-poisoning
//                                                           binding.
//   RV7 C_transfer = (v_s + v_r) * G + r_transfer * H       Credited
//                                                           commitment.
//   RV8 v_tilde    = (v_s + v_r)
//                      + Poseidon2(delta_transfer_amount, s, sigma)
//                                                           Encrypted amount
//                                                           (emitted).
//   RV9 r_e != 0                                            Rules out R_e = O
//                                                           and S, S_{a,r} = O.
// Recipient-auditor block (T_a1-T_a4 of the transfer circuit)
//   RV_a1 s_{a,r}  = ecdh(r_e, K_aud_r)
//   RV_a2 (m_v_r, m_r_r)
//                  = SpongeSqueeze_2(delta_aud_r, s_{a,r}, sigma)
//   RV_a3 v_tilde_aud_r = (v_s + v_r) + m_v_r
//   RV_a4 r_tilde_aud_r = r_transfer + m_r_r
//
// Point-validation doctrine (docs/protocol/proof-system.md#on-chain-point-arithmetic)
// ----------------------------------------
// C_spend, C_receive, C_transfer, and R_e are bound to in-circuit
// multi_scalar_mul outputs (RV1, RV2, RV7, RV4). PVK_B was constrained on-curve
// at the recipient's registration and is loaded from trusted storage (path
// (2)). K_aud_r is a public-input key consumed by an ECDH multiplication (path
// (3)), so it is validated on-curve and non-identity before RV_a1.
//
// Public inputs (17 fields, in docs/compliance.md#recovery-circuit canonical order)
// --------------------------------------------------------------------------------
//   Idx  Param          Symbol         Source / Note
//   ---  -----          ------         -----------------------------------
//   0    c_spend_x      C_spend.x      Loaded from the source's
//   1    c_spend_y      C_spend.y        `spendable_commitment`.
//   2    c_receive_x    C_receive.x    Loaded from the source's
//   3    c_receive_y    C_receive.y      `receiving_commitment`.
//   4    pvk_b_x        PVK_B.x        Loaded from the recipient's
//   5    pvk_b_y        PVK_B.y          `viewing_public_key`.
//   6    addr_f         addr_f         Loaded from instance storage.
//   7    k_aud_r_x      K_aud_r.x      Fetched from the auditor registry under
//   8    k_aud_r_y      K_aud_r.y        the recipient's `auditor_id`.
//   9    c_transfer_x   C_transfer.x   Prover-supplied; added to the
//   10   c_transfer_y   C_transfer.y     recipient's `receiving_commitment`.
//   11   r_e_x          R_e.x          Prover-supplied ephemeral key;
//   12   r_e_y          R_e.y            emitted.
//   13   v_tilde        v_tilde        Prover-supplied; emitted.
//   14   sigma          sigma          Prover-supplied random salt; emitted.
//   15   v_tilde_aud_r  v_tilde_aud_r  Prover-supplied; emitted.
//   16   r_tilde_aud_r  r_tilde_aud_r  Prover-supplied; emitted.
//
// `addr_f` is referenced by no gate: its presence in the public-input set binds
// the proof to one contract, on the `register/_acct_f` precedent.
//
// Private witnesses
// -----------------
//   v_s   Plaintext spendable-balance value.
//   r_s   Plaintext blinding factor for C_spend.
//   v_r   Plaintext receiving-balance value.
//   r_r   Plaintext blinding factor for C_receive.
//   r_e   Ephemeral scalar for recipient + auditor ECDH; must satisfy
//         r_e != 0 (RV9).

fn main(
    v_s: Field,
    r_s: Field,
    v_r: Field,
    r_r: Field,
    r_e: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    c_receive_x: pub Field,
    c_receive_y: pub Field,
    pvk_b_x: pub Field,
    pvk_b_y: pub Field,
    addr_f: pub Field,
    k_aud_r_x: pub Field,
    k_aud_r_y: pub Field,
    c_transfer_x: pub Field,
    c_transfer_y: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    v_tilde: pub Field,
    sigma: pub Field,
    v_tilde_aud_r: pub Field,
    r_tilde_aud_r: pub Field,
) {
    let _ = addr_f;

    // RV9 -- runs first so the r_e = 0 attack is rejected before any scalar
    // mul against it could quietly produce the identity.
    assert(r_e != 0);

    // RV1
    let c_spend_derived = commit(v_s, r_s);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // RV2
    let c_receive_derived = commit(v_r, r_r);
    assert(c_receive_derived.x == c_receive_x);
    assert(c_receive_derived.y == c_receive_y);

    // RV3
    v_s.assert_max_bit_size::<127>();
    v_r.assert_max_bit_size::<127>();
    let v = v_s + v_r;
    v.assert_max_bit_size::<127>();

    // RV4
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // RV5
    let pvk_b = EmbeddedCurvePoint { x: pvk_b_x, y: pvk_b_y, is_infinite: false };
    let s = ecdh(r_e, pvk_b);

    // RV6
    let r_transfer = derive_transfer_blind(s, sigma);

    // RV7
    let c_transfer_derived = commit(v, r_transfer);
    assert(c_transfer_derived.x == c_transfer_x);
    assert(c_transfer_derived.y == c_transfer_y);

    // RV8
    assert(encrypt_amount(v, s, sigma) == v_tilde);

    // K_aud_r point validation (path (3)).
    let k_aud_r = EmbeddedCurvePoint { x: k_aud_r_x, y: k_aud_r_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_r);

    // RV_a1
    let s_a_r = ecdh(r_e, k_aud_r);

    // RV_a2
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r, sigma);

    // RV_a3
    assert(v + m_r[0] == v_tilde_aud_r);

    // RV_a4
    assert(r_transfer + m_r[1] == r_tilde_aud_r);
}
