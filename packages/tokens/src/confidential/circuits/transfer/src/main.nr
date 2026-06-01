use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_spend_r, derive_tx_blind, domain, ecdh,
    encrypt_amount, encrypt_balance, scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// Transfer circuit -- design doc Section 7.6.
//
// Constraints
// -----------
// Sender state (balance conservation + ownership)
//   T1  Y_A         = sk_A * H                              Sender key
//                                                           ownership.
//   T2  vk_A        = Poseidon2(delta_vk, sk_A, addr_f)       Contract-bound
//                                                           viewing key.
//   T3  C_spend^A   = v_A * G + r_A * H                     Opening of the
//                                                           sender's current
//                                                           spendable balance.
//   T4  v_A, v_tx, v_A - v_tx in [0, 2^127)                 Range validity
//                                                           (Section 2.6); the
//                                                           [0, 2^127) bound is
//                                                           the SEP-41 i128
//                                                           non-negative range
//                                                           (Section 3.4).
//   T10 r_A'        = Poseidon2(delta_spend_r, vk_A, sigma) Deterministic
//                                                           randomness for the
//                                                           new sender balance.
//   T11 C_spend'    = (v_A - v_tx) * G + r_A' * H           New spendable
//                                                           commitment.
//   T12 b_tilde     = (v_A - v_tx)
//                       + Poseidon2(delta_enc_bal, vk_A, sigma)
//                                                           Encrypted balance
//                                                           scalar (emitted).
// Recipient ECDH + transfer commitment (Section 5.3, 5.4)
//   T5  S           = r_e * PVK_B                           Recipient ECDH
//                                                           shared secret.
//   T6  R_e         = r_e * H                               Ephemeral public
//                                                           key.
//   T7  r_tx        = Poseidon2(delta_tx_blind, S.x, sigma) Anti-poisoning
//                                                           binding: r_tx is
//                                                           uniquely determined
//                                                           by (S.x, sigma).
//   T8  C_tx        = v_tx * G + r_tx * H                   Transfer
//                                                           commitment.
//   T9  v_tilde     = v_tx
//                       + Poseidon2(delta_tx_amount, S.x, sigma)
//                                                           Encrypted amount
//                                                           (emitted).
//   T13 r_e != 0                                            Rules out R_e = O
//                                                           and S, S_{a,r},
//                                                           S_{a,s} = O.
// Auditor block (dual-channel visibility, Section 8.1)
//   T_a1 S_{a,r}    = r_e * K_aud_r                         Recipient-auditor
//                                                           ECDH shared secret
//                                                           (reuses r_e).
//   T_a2 (m_v_r, m_r_r)
//                  = SpongeSqueeze_2(delta_aud_r,
//                                    S_{a,r}.x, sigma)      Recipient-channel
//                                                           sponge: two masks.
//   T_a3 v_tilde_aud_r = v_tx + m_v_r                       Recipient-auditor
//                                                           encrypted amount.
//   T_a4 r_tilde_aud_r = r_tx + m_r_r                       Recipient-auditor
//                                                           encrypted transfer
//                                                           randomness.
//   T_a5 S_{a,s}    = r_e * K_aud_s                         Sender-auditor ECDH
//                                                           shared secret
//                                                           (reuses r_e).
//   T_a6 (m_v_s, m_b_s)
//                  = SpongeSqueeze_2(delta_aud_s,
//                                    S_{a,s}.x, sigma)      Sender-channel
//                                                           sponge: two masks.
//   T_a7 v_tilde_aud_s = v_tx + m_v_s                       Sender-auditor
//                                                           encrypted amount.
//   T_a8 b_tilde_aud_s = (v_A - v_tx) + m_b_s               Sender-auditor
//                                                           encrypted balance
//                                                           checkpoint.
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// Y_A, C_spend', C_tx, R_e, S, S_{a,r}, and S_{a,s} are bound to in-circuit
// multi_scalar_mul outputs (T1, T11, T8, T6, T5, T_a1, T_a5) and are therefore
// on-curve by construction -- no explicit check needed. PVK_B was constrained
// on-curve when the recipient registered (R3, Section 7.2) and is loaded by
// the contract from trusted storage, so it is path (2) of Section 10.8 and the
// contract trusts it on read. K_aud_r and K_aud_s are public-input keys
// consumed by ECDH multiplications (path (3)); the verifier doesn't check
// curve membership and an off-curve auditor key would break the soundness of
// T_a1 / T_a5, so this file explicitly validates both on-curve AND
// non-identity before the auditor block runs.
//
// Public inputs (24 fields, in design-doc canonical order)
// --------------------------------------------------------
//   Idx  Param            Symbol          Source / Note
//   ---  -----            ------          -----------------------------------
//   0    c_spend_x        C_spend^A.x     Loaded from `from.spendable_balance`.
//   1    c_spend_y        C_spend^A.y
//   2    y_x              Y_A.x           Loaded from `from.spending_key`.
//   3    y_y              Y_A.y
//   4    pvk_b_x          PVK_B.x         Loaded from `to.viewing_public_key`.
//   5    pvk_b_y          PVK_B.y           Recipient must be registered.
//   6    addr_f             addr_f            `env.current_contract_address()`.
//   7    k_aud_r_x        K_aud_r.x       Fetched from the auditor contract by
//   8    k_aud_r_y        K_aud_r.y         `to.auditor_id`.
//   9    k_aud_s_x        K_aud_s.x       Fetched from the auditor contract by
//   10   k_aud_s_y        K_aud_s.y         `from.auditor_id`.
//   11   c_spend_new_x    C_spend'.x      Prover-supplied; written to
//   12   c_spend_new_y    C_spend'.y        `from.spendable_balance`.
//   13   c_tx_x           C_tx.x          Prover-supplied; added to
//   14   c_tx_y           C_tx.y            `to.receiving_balance`.
//   15   r_e_x            R_e.x           Prover-supplied ephemeral key;
//   16   r_e_y            R_e.y             emitted.
//   17   v_tilde          v_tilde         Prover-supplied encrypted amount;
//                                         emitted.
//   18   b_tilde          b_tilde         Prover-supplied encrypted balance
//                                         scalar; emitted.
//   19   sigma            sigma           Prover-supplied random salt; emitted.
//   20   v_tilde_aud_r    v_tilde_aud_r   Prover-supplied recipient-auditor
//                                         encrypted amount; emitted.
//   21   r_tilde_aud_r    r_tilde_aud_r   Prover-supplied recipient-auditor
//                                         encrypted transfer randomness;
//                                         emitted.
//   22   v_tilde_aud_s    v_tilde_aud_s   Prover-supplied sender-auditor
//                                         encrypted amount; emitted.
//   23   b_tilde_aud_s    b_tilde_aud_s   Prover-supplied sender-auditor
//                                         encrypted balance checkpoint;
//                                         emitted.
//
// Private witnesses
// -----------------
//   sk     Sender's spending secret scalar.
//   v      Plaintext sender spendable-balance value.
//   r      Plaintext blinding factor for C_spend^A.
//   v_tx   Plaintext transfer amount.
//   r_e    Ephemeral scalar for recipient + auditor ECDH; must satisfy
//          r_e != 0 (T13).

fn main(
    sk: Field,
    v: Field,
    r: Field,
    v_tx: Field,
    r_e: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    y_x: pub Field,
    y_y: pub Field,
    pvk_b_x: pub Field,
    pvk_b_y: pub Field,
    addr_f: pub Field,
    k_aud_r_x: pub Field,
    k_aud_r_y: pub Field,
    k_aud_s_x: pub Field,
    k_aud_s_y: pub Field,
    c_spend_new_x: pub Field,
    c_spend_new_y: pub Field,
    c_tx_x: pub Field,
    c_tx_y: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    v_tilde: pub Field,
    b_tilde: pub Field,
    sigma: pub Field,
    v_tilde_aud_r: pub Field,
    r_tilde_aud_r: pub Field,
    v_tilde_aud_s: pub Field,
    b_tilde_aud_s: pub Field,
) {
    // T13 -- runs first so the r_e = 0 attack is rejected before any
    // scalar mul against it could quietly produce the identity.
    assert(r_e != 0);

    // T1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // T2
    let vk = vk_from_sk(sk, addr_f);

    // T3
    let c_spend_derived = commit(v, r);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // T4 -- Section 2.6 spells out the 127-bit decomposition / recomposition
    // pattern; `assert_max_bit_size` is the Noir stdlib primitive that
    // implements it directly.
    v.assert_max_bit_size::<127>();
    v_tx.assert_max_bit_size::<127>();
    let v_new = v - v_tx;
    v_new.assert_max_bit_size::<127>();

    // T6
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // T5 -- PVK_B is path (2) of Section 10.8 (proof-constrained at the
    // recipient's registration, trusted on read).
    let pvk_b = EmbeddedCurvePoint { x: pvk_b_x, y: pvk_b_y, is_infinite: false };
    let s_x = ecdh(r_e, pvk_b);

    // T7 (anti-poisoning, Section 5.4)
    let r_tx = derive_tx_blind(s_x, sigma);

    // T8
    let c_tx_derived = commit(v_tx, r_tx);
    assert(c_tx_derived.x == c_tx_x);
    assert(c_tx_derived.y == c_tx_y);

    // T9
    let v_tilde_derived = encrypt_amount(v_tx, s_x, sigma);
    assert(v_tilde_derived == v_tilde);

    // T10
    let r_new = derive_spend_r(vk, sigma);

    // T11
    let c_spend_new_derived = commit(v_new, r_new);
    assert(c_spend_new_derived.x == c_spend_new_x);
    assert(c_spend_new_derived.y == c_spend_new_y);

    // T12
    let b_tilde_derived = encrypt_balance(v_new, vk, sigma);
    assert(b_tilde_derived == b_tilde);

    // K_aud_r / K_aud_s point validation (Section 10.8: public-input keys).
    let k_aud_r = EmbeddedCurvePoint { x: k_aud_r_x, y: k_aud_r_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_r);
    let k_aud_s = EmbeddedCurvePoint { x: k_aud_s_x, y: k_aud_s_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_s);

    // T_a1 (recipient-auditor shared secret x-coordinate)
    let s_a_r_x = ecdh(r_e, k_aud_r);

    // T_a2 (recipient-channel masks: amount, then r_tx)
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, sigma);

    // T_a3
    assert(v_tx + m_r[0] == v_tilde_aud_r);

    // T_a4
    assert(r_tx + m_r[1] == r_tilde_aud_r);

    // T_a5 (sender-auditor shared secret x-coordinate)
    let s_a_s_x = ecdh(r_e, k_aud_s);

    // T_a6 (sender-channel masks: amount, then balance)
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, sigma);

    // T_a7
    assert(v_tx + m_s[0] == v_tilde_aud_s);

    // T_a8
    assert(v_new + m_s[1] == b_tilde_aud_s);
}
