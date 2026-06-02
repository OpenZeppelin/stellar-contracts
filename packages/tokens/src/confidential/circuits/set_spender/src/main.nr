use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_allow_r, derive_spend_r, domain,
    dvk_from_vk_op, ecdh, encrypt_allowance, encrypt_balance, encrypt_esc_dvk, scalar_mul,
    sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// SetSpender circuit -- design doc Section 7.7.
//
// Constraints
// -----------
// Owner balance split (ownership + new spendable balance)
//   S1  Y          = sk * H                              Owner key
//                                                        ownership.
//   S2  vk         = Poseidon2(delta_vk, sk, addr_f)       Contract-bound
//                                                        viewing key.
//   S3  C_spend    = v * G + r * H                       Opening of current
//                                                        spendable balance.
//   S4  v, v_a, v - v_a in [0, 2^127)                    Range validity
//                                                        (Section 2.6); the
//                                                        [0, 2^127) bound is
//                                                        the SEP-41 i128
//                                                        non-negative range
//                                                        (Section 3.4).
//   S9  r'         = Poseidon2(delta_spend_r, vk, sigma) Deterministic
//                                                        randomness for the
//                                                        new spendable
//                                                        balance.
//   S10 C_spend'   = (v - v_a) * G + r' * H              New spendable
//                                                        commitment.
//   S11 b_tilde    = (v - v_a)
//                      + Poseidon2(delta_enc_bal, vk, sigma)
//                                                        Encrypted balance
//                                                        scalar (emitted).
// Delegation + allowance (per-spender escrow)
//   S5  dvk_i      = Poseidon2(delta_dvk, vk, op_i)      Delegation viewing
//                                                        key; contract-bound
//                                                        via vk.
//   S6  r_a        = Poseidon2(delta_allow_r, dvk_i,
//                              sigma_a)                  Deterministic
//                                                        allowance blinding.
//   S7  C_a        = v_a * G + r_a * H                   Allowance commitment.
//   S8  a_tilde    = v_a
//                      + Poseidon2(delta_enc_allow, dvk_i, sigma_a)
//                                                        Encrypted allowance
//                                                        scalar (emitted).
//   S12 Escrowed dvk_i correctly encrypts under Y_op via ECDH.
//       Expanded per Section 7.11 into three sub-constraints over the
//       prover-supplied escrowed_dvk = (R_x, dvk_cipher):
//         (a) R_x          = R_e.x                       Same r_e as the
//                                                        auditor block
//                                                        (S_a1), forced
//                                                        equal here.
//         (b) s_esc        = (r_e * Y_op).x              Spender-handoff
//                                                        ECDH shared secret.
//         (c) dvk_cipher   = dvk_i
//                              + Poseidon2(delta_esc_dvk, s_esc, op_i)
//                                                        Masked dvk_i for the
//                                                        spender.
//   S13 r_e != 0                                         Rules out R_e = O,
//                                                        S_{a,s} = O, and a
//                                                        trivial escrow
//                                                        shared secret
//                                                        (same r_e is reused
//                                                        for the dvk escrow
//                                                        ECDH; Section 7.11).
// Auditor block (owner-auditor visibility, Section 8.1)
//   S_a1 R_e        = r_e * H                            Ephemeral key for
//                                                        auditor ECDH (also
//                                                        the escrow ephemeral
//                                                        key; see S12(a)).
//   S_a2 S_{a,s}    = r_e * K_aud_s                      Owner-auditor ECDH
//                                                        shared secret.
//   S_a3 (m_v, m_b) = SpongeSqueeze_2(delta_aud_s,
//                                    S_{a,s}.x, sigma)   Owner-channel
//                                                        sponge: two masks.
//   S_a4 v_tilde_aud_s = v_a + m_v                       Owner-auditor
//                                                        encrypted escrow
//                                                        amount.
//   S_a5 b_tilde_aud_s = (v - v_a) + m_b                 Owner-auditor
//                                                        encrypted balance
//                                                        checkpoint.
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// Y, C_spend, C_spend', C_a, R_e, and S_{a,s} are bound to in-circuit
// multi_scalar_mul outputs (S1, S3, S10, S7, S_a1, S_a2) and are therefore
// on-curve by construction -- no explicit check needed. Y_op was constrained
// on-curve when the spender registered (R1, Section 7.2) and is loaded by
// the contract from trusted storage, so it is path (2) of Section 10.8 and the
// contract trusts it on read. K_aud_s is a public-input key consumed by an
// ECDH multiplication (path (3)); the verifier doesn't check curve membership
// and an off-curve K_aud_s would break the soundness of S_a2, so this file
// explicitly validates it on-curve AND non-identity before the auditor block
// runs.
//
// Public inputs (24 fields, in design-doc canonical order)
// --------------------------------------------------------
//   Idx  Param                Symbol            Source / Note
//   ---  -----                ------            -----------------------------
//   0    c_spend_x            C_spend.x         Loaded from
//   1    c_spend_y            C_spend.y           `from.spendable_balance`.
//   2    y_x                  Y.x               Loaded from
//   3    y_y                  Y.y                 `from.spending_key`.
//   4    y_op_x               Y_op.x            Loaded from spender
//   5    y_op_y               Y_op.y              account's `spending_key`.
//                                                Spender must be registered.
//   6    op_i                 op_i              address_to_field(`spender`),
//                                                computed off-circuit by the
//                                                contract (Section 2.7).
//   7    addr_f                 addr_f              `env.current_contract_address()`.
//   8    k_aud_s_x            K_aud_s.x         Fetched from the auditor
//   9    k_aud_s_y            K_aud_s.y           contract by `from.auditor_id`.
//   10   c_spend_new_x        C_spend'.x        Prover-supplied; written to
//   11   c_spend_new_y        C_spend'.y          `from.spendable_balance`.
//   12   c_a_x                C_a.x             Prover-supplied; written to
//   13   c_a_y                C_a.y               `(from, op).allowance`.
//   14   escrowed_dvk_r_x     R_x               Prover-supplied; the first
//                                                32-byte limb of the on-chain
//                                                escrowed_dvk encoding
//                                                (Section 7.11); forced equal
//                                                to R_e.x by S12(a).
//   15   escrowed_dvk_cipher  dvk_cipher        Prover-supplied; second limb
//                                                of escrowed_dvk; stored.
//   16   b_tilde              b_tilde           Prover-supplied encrypted
//                                                balance scalar; emitted.
//   17   a_tilde              a_tilde           Prover-supplied encrypted
//                                                allowance scalar; emitted.
//   18   sigma                sigma             Prover-supplied owner-channel
//                                                salt; emitted.
//   19   sigma_a              sigma_a           Prover-supplied allowance
//                                                salt; stored.
//   20   r_e_x                R_e.x             Prover-supplied ephemeral key
//   21   r_e_y                R_e.y               for auditor ECDH; emitted.
//   22   v_tilde_aud_s        v_tilde_aud_s     Prover-supplied owner-auditor
//                                                encrypted escrow amount;
//                                                emitted.
//   23   b_tilde_aud_s        b_tilde_aud_s     Prover-supplied owner-auditor
//                                                encrypted balance checkpoint;
//                                                emitted.
//
// Private witnesses
// -----------------
//   sk   Owner spending secret scalar.
//   v    Plaintext spendable-balance value.
//   r    Plaintext blinding factor for C_spend.
//   v_a  Plaintext per-spender allowance.
//   r_e  Ephemeral scalar for the auditor + escrow ECDH; must satisfy
//        r_e != 0 (S13).

fn main(
    sk: Field,
    v: Field,
    r: Field,
    v_a: Field,
    r_e: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    y_x: pub Field,
    y_y: pub Field,
    y_op_x: pub Field,
    y_op_y: pub Field,
    op_i: pub Field,
    addr_f: pub Field,
    k_aud_s_x: pub Field,
    k_aud_s_y: pub Field,
    c_spend_new_x: pub Field,
    c_spend_new_y: pub Field,
    c_a_x: pub Field,
    c_a_y: pub Field,
    escrowed_dvk_r_x: pub Field,
    escrowed_dvk_cipher: pub Field,
    b_tilde: pub Field,
    a_tilde: pub Field,
    sigma: pub Field,
    sigma_a: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    v_tilde_aud_s: pub Field,
    b_tilde_aud_s: pub Field,
) {
    // S13 -- runs first so the r_e = 0 attack is rejected before any
    // scalar mul against it could quietly produce the identity (R_e, S_{a,s},
    // and the escrow shared secret all collapse together).
    assert(r_e != 0);

    // S1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // S2
    let vk = vk_from_sk(sk, addr_f);

    // S3
    let c_spend_derived = commit(v, r);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // S4 -- Section 2.6 spells out the 127-bit decomposition / recomposition
    // pattern; `assert_max_bit_size` is the Noir stdlib primitive that
    // implements it directly.
    v.assert_max_bit_size::<127>();
    v_a.assert_max_bit_size::<127>();
    let v_new = v - v_a;
    v_new.assert_max_bit_size::<127>();

    // S5
    let dvk = dvk_from_vk_op(vk, op_i);

    // S6
    let r_a = derive_allow_r(dvk, sigma_a);

    // S7
    let c_a_derived = commit(v_a, r_a);
    assert(c_a_derived.x == c_a_x);
    assert(c_a_derived.y == c_a_y);

    // S8
    let a_tilde_derived = encrypt_allowance(v_a, dvk, sigma_a);
    assert(a_tilde_derived == a_tilde);

    // S9
    let r_new = derive_spend_r(vk, sigma);

    // S10
    let c_spend_new_derived = commit(v_new, r_new);
    assert(c_spend_new_derived.x == c_spend_new_x);
    assert(c_spend_new_derived.y == c_spend_new_y);

    // S11
    let b_tilde_derived = encrypt_balance(v_new, vk, sigma);
    assert(b_tilde_derived == b_tilde);

    // S_a1
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // S12(a) -- escrowed_dvk's R_x limb is forced equal to R_e.x, which is
    // the auditor-block ephemeral. Section 7.11: "the escrow's R_x and the
    // auditor channel's R_e.x are forced equal" -- this constraint is what
    // forces it.
    assert(escrowed_dvk_r_x == r_e_x);

    // Y_op is path (2) of Section 10.8: bound to scalar_mul(sk_op, H) when
    // the spender registered (R1), trusted on read from the contract.
    let y_op = EmbeddedCurvePoint { x: y_op_x, y: y_op_y, is_infinite: false };

    // S12(b)
    let s_esc = ecdh(r_e, y_op);

    // S12(c)
    let escrowed_dvk_cipher_derived = encrypt_esc_dvk(dvk, s_esc, op_i);
    assert(escrowed_dvk_cipher_derived == escrowed_dvk_cipher);

    // K_aud_s point validation (Section 10.8: public-input key).
    let k_aud_s = EmbeddedCurvePoint { x: k_aud_s_x, y: k_aud_s_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_s);

    // S_a2 (owner-auditor shared secret x-coordinate)
    let s_a_s_x = ecdh(r_e, k_aud_s);

    // S_a3 (owner-channel masks: amount, then balance)
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, sigma);

    // S_a4
    assert(v_a + m_s[0] == v_tilde_aud_s);

    // S_a5
    assert(v_new + m_s[1] == b_tilde_aud_s);
}
