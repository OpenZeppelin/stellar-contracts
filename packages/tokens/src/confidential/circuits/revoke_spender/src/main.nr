use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_allow_r, derive_spend_r, domain,
    dvk_from_vk_op, ecdh, encrypt_balance, scalar_mul, sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// RevokeSpender circuit -- design doc Section 7.9.
//
// Constraints
// -----------
// Main block (owner ownership + delegation opening + balance reclaim)
//   V1  Y          = sk * H                              Owner key ownership.
//   V2  vk         = Poseidon2(delta_vk, sk, addr_f)       Contract-bound viewing
//                                                        key.
//   V3  dvk_i      = Poseidon2(delta_dvk, vk, op_i)      Delegation viewing
//                                                        key for this spender.
//   V4  C_a        = v_a * G + r_a * H, with
//        r_a       = Poseidon2(delta_allow_r, dvk_i, sigma_a)
//                                                        Opening of the
//                                                        on-chain allowance
//                                                        commitment (mirrors
//                                                        O3 in Section 7.8).
//   V5  C_spend    = v_s * G + r_s * H                   Opening of current
//                                                        spendable balance.
//   V9  v_s, v_a, v_s + v_a in [0, 2^127)                Range validity
//                                                        (Section 2.6); the
//                                                        sum check is the
//                                                        soundness-critical
//                                                        line of defense
//                                                        against a addr_f that
//                                                        would let the owner
//                                                        mint via field
//                                                        overflow.
//   V6  r'         = Poseidon2(delta_spend_r, vk, sigma) Deterministic
//                                                        randomness for the
//                                                        reclaimed balance.
//   V7  C_spend'   = (v_s + v_a) * G + r' * H            New spendable
//                                                        commitment with the
//                                                        allowance folded
//                                                        back in.
//   V8  b_tilde    = (v_s + v_a)
//                      + Poseidon2(delta_enc_bal, vk, sigma)
//                                                        Encrypted balance
//                                                        scalar (emitted).
//   V10 r_e != 0                                         Rules out R_e = O
//                                                        and S_{a,s} = O,
//                                                        which would collapse
//                                                        m_v and m_b to
//                                                        constant functions
//                                                        of sigma.
// Auditor block (owner-auditor visibility, Section 8.1 / 8.5)
//   V_a1 R_e       = r_e * H                             Ephemeral key for
//                                                        auditor ECDH.
//   V_a2 S_{a,s}   = r_e * K_{aud,s}                     Owner-auditor ECDH
//                                                        shared secret.
//   V_a3 (m_v, m_b)
//                  = SpongeSqueeze_2(delta_aud_s,
//                                    S_{a,s}.x, sigma)   Owner-channel sponge,
//                                                        two squeezes.
//   V_a4 v_tilde_aud_s = v_a + m_v                       Owner-auditor
//                                                        encrypted reclaimed
//                                                        amount (Section 8.5).
//   V_a5 b_tilde_aud_s = (v_s + v_a) + m_b               Owner-auditor
//                                                        encrypted post-reclaim
//                                                        balance checkpoint
//                                                        (Section 8.2).
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// Y, C_spend, C_a, C_spend', and R_e are each bound to in-circuit
// multi_scalar_mul outputs (V1, V5, V4, V7, V_a1) and are therefore on-curve
// by construction -- no explicit check needed. K_aud_s is a public-input key
// consumed by an ECDH multiplication; the verifier doesn't check curve
// membership, so an off-curve K_aud_s would break the soundness of V_a2.
// This file explicitly validates K_aud_s on-curve AND non-identity before
// V_a2 (mirrors withdraw).
//
// Public inputs (19 fields, in design-doc canonical order, Section 7.9)
// ---------------------------------------------------------------------
//   Idx  Param            Symbol         Source / Note
//   ---  -----            ------         -------------------------------------
//   0    c_spend_x        C_spend.x      Loaded from owner's
//   1    c_spend_y        C_spend.y        `spendable_balance`.
//   2    c_a_x            C_a.x          Loaded from the (account, spender)
//   3    c_a_y            C_a.y            delegation entry.
//   4    sigma_a          sigma_a        Loaded from the delegation entry
//                                          (`allowance_salt`).
//   5    y_x              Y.x            Loaded from owner's `spending_key`.
//   6    y_y              Y.y
//   7    op_i             op_i           `address_to_field(spender)`,
//                                          computed per-call (Section 2.7).
//   8    addr_f             addr_f           `env.current_contract_address()`.
//   9    k_aud_s_x        K_aud_s.x      Fetched from the auditor contract
//   10   k_aud_s_y        K_aud_s.y        by owner's `auditor_id`.
//   11   c_spend_new_x    C_spend'.x     Prover-supplied; written to
//   12   c_spend_new_y    C_spend'.y       owner's `spendable_balance` on
//                                          success.
//   13   b_tilde          b_tilde        Prover-supplied encrypted balance
//                                          scalar; emitted.
//   14   sigma            sigma          Prover-supplied random salt;
//                                          emitted.
//   15   r_e_x            R_e.x          Prover-supplied ephemeral key for
//   16   r_e_y            R_e.y            auditor ECDH; emitted.
//   17   v_tilde_aud_s    v_tilde_aud_s  Prover-supplied owner-auditor
//                                          encrypted reclaimed amount;
//                                          emitted.
//   18   b_tilde_aud_s    b_tilde_aud_s  Prover-supplied owner-auditor
//                                          encrypted balance checkpoint;
//                                          emitted.
//
// Private witnesses
// -----------------
//   sk    Spending secret scalar.
//   v_a   Plaintext escrowed-allowance value (loaded by the prover via dvk_i).
//   r_a   Plaintext blinding factor for C_a (bound to V4 via derive_allow_r).
//   v_s   Plaintext spendable-balance value.
//   r_s   Plaintext blinding factor for C_spend (see Section 10.4 on the
//         acknowledged 2^-127-per-merge case affecting r_s).
//   r_e   Ephemeral scalar for the auditor ECDH; must satisfy r_e != 0 (V10).

fn main(
    sk: Field,
    v_a: Field,
    r_a: Field,
    v_s: Field,
    r_s: Field,
    r_e: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    c_a_x: pub Field,
    c_a_y: pub Field,
    sigma_a: pub Field,
    y_x: pub Field,
    y_y: pub Field,
    op_i: pub Field,
    addr_f: pub Field,
    k_aud_s_x: pub Field,
    k_aud_s_y: pub Field,
    c_spend_new_x: pub Field,
    c_spend_new_y: pub Field,
    b_tilde: pub Field,
    sigma: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    v_tilde_aud_s: pub Field,
    b_tilde_aud_s: pub Field,
) {
    // V10 -- runs first so the r_e = 0 attack is rejected before any
    // scalar mul against it could quietly produce the identity.
    assert(r_e != 0);

    // V1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // V2
    let vk = vk_from_sk(sk, addr_f);

    // V3
    let dvk_i = dvk_from_vk_op(vk, op_i);

    // V4 -- mirrors O3 (Section 7.8): r_a is bound to (dvk_i, sigma_a), so
    // the only valid v_a is the one whose commitment matches the on-chain
    // C_a under that derived r_a.
    let r_a_derived = derive_allow_r(dvk_i, sigma_a);
    assert(r_a_derived == r_a);
    let c_a_derived = commit(v_a, r_a);
    assert(c_a_derived.x == c_a_x);
    assert(c_a_derived.y == c_a_y);

    // V5
    let c_spend_derived = commit(v_s, r_s);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // V9 -- Section 2.6 spells out the 127-bit decomposition / recomposition
    // pattern; `assert_max_bit_size` is the Noir stdlib primitive that
    // implements it directly. The sum check `v_s + v_a < 2^127` is the
    // soundness-critical step: without it, a prover could choose v_s and v_a
    // each individually in range but whose sum wraps mod F_r, effectively
    // minting tokens out of nothing on the reclaim path.
    v_s.assert_max_bit_size::<127>();
    v_a.assert_max_bit_size::<127>();
    let v_new = v_s + v_a;
    v_new.assert_max_bit_size::<127>();

    // V6
    let r_new = derive_spend_r(vk, sigma);

    // V7
    let c_spend_new_derived = commit(v_new, r_new);
    assert(c_spend_new_derived.x == c_spend_new_x);
    assert(c_spend_new_derived.y == c_spend_new_y);

    // V8
    let b_tilde_derived = encrypt_balance(v_new, vk, sigma);
    assert(b_tilde_derived == b_tilde);

    // V_a1
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // K_aud_s point validation (Section 10.8: public-input key).
    let k_aud_s = EmbeddedCurvePoint { x: k_aud_s_x, y: k_aud_s_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_s);

    // V_a2 (shared secret x-coordinate)
    let s_a_s_x = ecdh(r_e, k_aud_s);

    // V_a3 (owner-channel masks: reclaimed amount, then balance checkpoint)
    let m = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, sigma);

    // V_a4
    assert(v_a + m[0] == v_tilde_aud_s);

    // V_a5
    assert(v_new + m[1] == b_tilde_aud_s);
}
