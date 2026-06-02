use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_spend_r, ecdh, encrypt_auditor_sender_balance,
    encrypt_balance, scalar_mul, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// Withdraw circuit -- design doc Section 7.5.
//
// Constraints
// -----------
// Main block (balance conservation + ownership)
//   W1 Y         = sk * H                              Owner key ownership.
//   W2 vk        = Poseidon2(delta_vk, sk, addr_f)       Contract-bound viewing
//                                                      key.
//   W3 C_spend   = v * G + r * H                       Opening of current
//                                                      spendable balance.
//   W4 v, a, v - a in [0, 2^127)                       Range validity
//                                                      (Section 2.6); the
//                                                      [0, 2^127) bound is
//                                                      the SEP-41 i128
//                                                      non-negative range
//                                                      (Section 3.4)
//   W5 r'        = Poseidon2(delta_spend_r, vk, sigma) Deterministic
//                                                      randomness for the
//                                                      new balance.
//   W6 C_spend'  = (v - a) * G + r' * H                New spendable
//                                                      commitment.
//   W7 b_tilde   = (v - a)
//                    + Poseidon2(delta_enc_bal, vk, sigma)
//                                                      Encrypted balance
//                                                      scalar (emitted).
//   W8 r_e != 0                                        Rules out R_e = O and
//                                                      S_{a,s} = O, which
//                                                      would collapse m_b to
//                                                      a constant function
//                                                      of sigma.
// Auditor block (sender-auditor visibility, Section 8.1)
//   W_a1 R_e         = r_e * H                         Ephemeral key for
//                                                      auditor ECDH.
//   W_a2 S_{a,s}     = r_e * K_{aud,s}                 Sender-auditor ECDH
//                                                      shared secret.
//   W_a3 m_b         = SpongeSqueeze_1(delta_aud_s,
//                                     S_{a,s}.x, sigma) Sender-channel
//                                                      sponge, single squeeze.
//   W_a4 b_tilde_aud_s = (v - a) + m_b                  Sender-auditor
//                                                      encrypted balance
//                                                      checkpoint (emitted).
//
// W_a3 + W_a4 are encapsulated by `encrypt_auditor_sender_balance` in the lib.
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// Y, C_spend, C_spend', and R_e are each bound to in-circuit multi_scalar_mul
// outputs (W1, W3, W6, W_a1) and are therefore on-curve by construction --
// no explicit check needed. K_aud_s is a public-input key consumed by an
// ECDH multiplication; the verifier doesn't check curve membership, so an
// off-curve K_aud_s would break the soundness of W_a2. This file explicitly
// validates K_aud_s on-curve AND non-identity before W_a2.
//
// Public inputs (15 fields, in design-doc canonical order)
// --------------------------------------------------------
//   Idx  Param          Symbol        Source / Note
//   ---  -----          ------        -------------------------------------
//   0    c_spend_x      C_spend.x     Loaded from `from.spendable_balance`.
//   1    c_spend_y      C_spend.y
//   2    y_x            Y.x           Loaded from `from.spending_key`.
//   3    y_y            Y.y
//   4    addr_f           addr_f          `env.current_contract_address()`.
//   5    k_aud_s_x      K_aud_s.x     Fetched from the auditor contract by
//   6    k_aud_s_y     K_aud_s.y       `from.auditor_id`.
//   7    a              a             Public withdrawal amount; the contract
//                                     pre-checks a >= 0 at the entrypoint
//                                     (Section 3.4), W4 then closes
//                                     a < 2^127 in-circuit.
//   8    c_spend_new_x  C_spend'.x    Prover-supplied; written to
//   9    c_spend_new_y  C_spend'.y      `from.spendable_balance` on success.
//   10   sigma          sigma         Prover-supplied random salt; emitted.
//   11   b_tilde        b_tilde       Prover-supplied encrypted balance
//                                     scalar; emitted.
//   12   r_e_x          R_e.x         Prover-supplied ephemeral key for
//   13   r_e_y          R_e.y           auditor ECDH; emitted.
//   14   b_tilde_aud_s  b_tilde_aud_s Prover-supplied sender-auditor
//                                     encrypted balance checkpoint; emitted.
//
// Private witnesses
// -----------------
//   sk    Spending secret scalar.
//   v     Plaintext spendable-balance value.
//   r     Plaintext blinding factor for C_spend.
//   r_e   Ephemeral scalar for the auditor ECDH; must satisfy r_e != 0
//         (W8).

fn main(
    sk: Field,
    v: Field,
    r: Field,
    r_e: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    y_x: pub Field,
    y_y: pub Field,
    addr_f: pub Field,
    k_aud_s_x: pub Field,
    k_aud_s_y: pub Field,
    a: pub Field,
    c_spend_new_x: pub Field,
    c_spend_new_y: pub Field,
    sigma: pub Field,
    b_tilde: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    b_tilde_aud_s: pub Field,
) {
    // W8 -- runs first so the r_e = 0 attack is rejected before any
    // scalar mul against it could quietly produce the identity.
    assert(r_e != 0);

    // W1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // W2
    let vk = vk_from_sk(sk, addr_f);

    // W3
    let c_spend_derived = commit(v, r);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // W4 -- Section 2.6 spells out the 127-bit decomposition / recomposition
    // pattern; `assert_max_bit_size` is the Noir stdlib primitive that
    // implements it directly.
    v.assert_max_bit_size::<127>();
    a.assert_max_bit_size::<127>();
    let v_new = v - a;
    v_new.assert_max_bit_size::<127>();

    // W5
    let r_new = derive_spend_r(vk, sigma);

    // W6
    let c_spend_new_derived = commit(v_new, r_new);
    assert(c_spend_new_derived.x == c_spend_new_x);
    assert(c_spend_new_derived.y == c_spend_new_y);

    // W7
    let b_tilde_derived = encrypt_balance(v_new, vk, sigma);
    assert(b_tilde_derived == b_tilde);

    // W_a1
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // K_aud_s point validation (Section 10.8: public-input key).
    let k_aud_s = EmbeddedCurvePoint { x: k_aud_s_x, y: k_aud_s_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_s);

    // W_a2 (shared secret x-coordinate)
    let s_a_s_x = ecdh(r_e, k_aud_s);

    // W_a3 + W_a4
    let b_tilde_aud_s_derived = encrypt_auditor_sender_balance(v_new, s_a_s_x, sigma);
    assert(b_tilde_aud_s_derived == b_tilde_aud_s);
}
