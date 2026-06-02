use stellar_confidential_lib::{
    H, assert_on_curve_non_identity, commit, derive_allow_r, derive_tx_blind, domain, ecdh,
    encrypt_allowance, encrypt_amount, scalar_mul, sponge_squeeze_2,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;

mod tests;

// SpenderTransfer circuit -- design doc Section 7.8.
//
// Constraints
// -----------
// Spender key ownership
//   O1  Y_op       = sk_op * H                              Spender key
//                                                           ownership.
// Allowance state (current)
//   O2  C_a        = v_a * G + r_a * H                      Opening of the
//                                                           stored allowance
//                                                           commitment.
//   O3  r_a        = Poseidon2(delta_allow_r, dvk_i,
//                              sigma_a)                     Allowance
//                                                           randomness matches
//                                                           stored state (the
//                                                           contract-binding
//                                                           anchor, see below).
//   O4  v_a, v_tx, v_a - v_tx in [0, 2^127)                 Range validity
//                                                           (Section 2.6); the
//                                                           [0, 2^127) bound is
//                                                           the SEP-41 i128
//                                                           non-negative range
//                                                           (Section 3.4).
// Recipient ECDH + transfer commitment (Section 5.3, 5.4)
//   O5  S          = r_e * PVK_recipient                    Recipient ECDH
//                                                           shared secret.
//   O6  R_e        = r_e * H                                Ephemeral public
//                                                           key.
//   O7  r_tx       = Poseidon2(delta_tx_blind, S.x,
//                              sigma_a)                     Anti-poisoning
//                                                           binding for the
//                                                           transfer blinding.
//   O8  C_tx       = v_tx * G + r_tx * H                    Transfer
//                                                           commitment.
//   O9  v_tilde    = v_tx
//                       + Poseidon2(delta_tx_amount, S.x,
//                                   sigma_a)                Encrypted amount
//                                                           (emitted).
// Allowance state (new)
//   --  sigma_a' != sigma_a                                 Salt rotation
//                                                           (implementation
//                                                           hardening; not in
//                                                           Section 7.8).
//   O10 r_a'       = Poseidon2(delta_allow_r, dvk_i,
//                              sigma_a')                    New allowance
//                                                           randomness.
//   O11 C_a'       = (v_a - v_tx) * G + r_a' * H            New allowance
//                                                           commitment.
//   O12 a_tilde'   = (v_a - v_tx)
//                       + Poseidon2(delta_enc_allow, dvk_i,
//                                   sigma_a')               Encrypted new
//                                                           allowance scalar
//                                                           (emitted).
// Nonzero ephemeral
//   O13 r_e != 0                                            Rules out R_e = O
//                                                           and S, S_{a,r},
//                                                           S_{a,s} = O.
// Auditor block (dual-channel visibility, Section 8.1 + Section 8.4)
//   O_a1 S_{a,r}   = r_e * K_aud_r                          Recipient-auditor
//                                                           ECDH shared secret
//                                                           (reuses r_e).
//   O_a2 (m_v_r, m_r_r)
//                  = SpongeSqueeze_2(delta_aud_r,
//                                    S_{a,r}.x, sigma_a)    Recipient-channel
//                                                           sponge: two masks.
//   O_a3 v_tilde_aud_r = v_tx + m_v_r                       Recipient-auditor
//                                                           encrypted amount.
//   O_a4 r_tilde_aud_r = r_tx + m_r_r                       Recipient-auditor
//                                                           encrypted transfer
//                                                           randomness.
//   O_a5 S_{a,s}   = r_e * K_aud_s                          Owner-auditor ECDH
//                                                           shared secret
//                                                           (reuses r_e); the
//                                                           sender-channel
//                                                           visibility points
//                                                           at the funds'
//                                                           OWNER, not the
//                                                           spender (Section
//                                                           8.4).
//   O_a6 (m_v_s, m_a_s)
//                  = SpongeSqueeze_2(delta_aud_s,
//                                    S_{a,s}.x, sigma_a)    Owner-channel
//                                                           sponge: two masks.
//   O_a7 v_tilde_aud_s = v_tx + m_v_s                       Owner-auditor
//                                                           encrypted amount.
//   O_a8 a_tilde_aud_s = (v_a - v_tx) + m_a_s               Owner-auditor
//                                                           encrypted
//                                                           post-transfer
//                                                           allowance.
//
// Contract binding (Section 7.8 final paragraph)
// ---------------------------------------------
// Unlike owner-initiated circuits, SpenderTransfer does NOT constrain the vk
// derivation -- the spender has no access to the owner's sk. Contract binding
// is inherited indirectly through the allowance commitment chain: SetSpender
// derived `dvk_i` from the contract-specific `vk` (S2, S5), which determined
// `r_a` (S6) and thus `C_a` (S7). Here O3 verifies the spender's claimed
// `dvk_i` against the on-chain `C_a` via `sigma_a`. Because `C_a` is a public
// input and was constructed with contract-specific randomness, a proof
// generated against one contract's `C_a` cannot verify against another's.
//
// Channel-nonce reuse (Section 6.2 *Dual role*)
// ---------------------------------------------
// `sigma_a` serves as the freshness nonce for THREE distinct uses in this
// circuit: the allowance-randomness Poseidon (O3), the recipient ECDH chain
// (O7 / O9), and both auditor-channel sponges (O_a2 / O_a6). Soundness derives
// from ECDH shared-secret unpredictability (S.x is unknown to anyone but the
// recipient and the prover); `sigma_a` itself is public and need not be
// secret. Only the new-allowance constraints O10 and O12 use the fresh
// `sigma_a'`.
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// Y_op, C_a, C_a', C_tx, R_e, S, S_{a,r}, and S_{a,s} are bound to in-circuit
// multi_scalar_mul outputs (O1, O2, O11, O8, O6, O5, O_a1, O_a5) and are
// therefore on-curve by construction -- no explicit check needed.
// PVK_recipient was constrained on-curve when the recipient registered (R3,
// Section 7.2) and is loaded by the contract from trusted storage, so it is
// path (2) of Section 10.8 and the circuit consumes it without re-checking.
// K_aud_r and K_aud_s are public-input keys with no proof constraint at
// insertion (path (3)) -- the verifier doesn't check curve membership and
// an off-curve key would break the soundness of O_a1 / O_a5. This file
// explicitly validates both on-curve AND non-identity before the
// corresponding ECDH consumes them.
//
// Public inputs (24 fields, in design-doc canonical order)
// --------------------------------------------------------
//   Idx  Param                Symbol            Source / Note
//   ---  -----                ------            -------------------------------
//   0    c_a_x                C_a.x             Loaded from the
//   1    c_a_y                C_a.y               `(from, spender)` delegation
//                                                 entry's allowance_commitment.
//   2    sigma_a              sigma_a           Loaded from the delegation
//                                                 entry's allowance_salt.
//   3    y_op_x               Y_op.x            Loaded from
//   4    y_op_y               Y_op.y              `spender.spending_key`;
//                                                 matches the auth principal.
//   5    pvk_recipient_x      PVK_recipient.x   Loaded from
//   6    pvk_recipient_y      PVK_recipient.y     `to.viewing_public_key`.
//                                                 Recipient must be registered.
//   7    k_aud_r_x            K_aud_r.x         Fetched from the auditor
//   8    k_aud_r_y            K_aud_r.y           contract by `to.auditor_id`.
//   9    k_aud_s_x            K_aud_s.x         Fetched from the auditor
//   10   k_aud_s_y            K_aud_s.y           contract by the OWNER's
//                                                 `auditor_id`, not the
//                                                 spender's (Section 7.8).
//   11   c_a_new_x            C_a'.x            Prover-supplied; written to
//   12   c_a_new_y            C_a'.y              the delegation entry's new
//                                                 allowance_commitment.
//   13   c_tx_x               C_tx.x            Prover-supplied; added to
//   14   c_tx_y               C_tx.y              `to.receiving_balance`.
//   15   r_e_x                R_e.x             Prover-supplied ephemeral key;
//   16   r_e_y                R_e.y               emitted.
//   17   v_tilde              v_tilde           Prover-supplied encrypted
//                                                 transfer amount; emitted.
//   18   a_tilde_new          a_tilde'          Prover-supplied encrypted new
//                                                 allowance scalar; stored.
//   19   sigma_a_new          sigma_a'          Prover-supplied fresh salt;
//                                                 written to allowance_salt.
//   20   v_tilde_aud_r        v_tilde_aud_r     Prover-supplied recipient-
//                                                 auditor encrypted amount;
//                                                 emitted.
//   21   r_tilde_aud_r        r_tilde_aud_r     Prover-supplied recipient-
//                                                 auditor encrypted transfer
//                                                 randomness; emitted.
//   22   v_tilde_aud_s        v_tilde_aud_s     Prover-supplied owner-auditor
//                                                 encrypted amount; emitted.
//   23   a_tilde_aud_s        a_tilde_aud_s     Prover-supplied owner-auditor
//                                                 encrypted post-transfer
//                                                 allowance; emitted.
//
// Private witnesses
// -----------------
//   sk_op   Spender's spending secret scalar.
//   dvk_i   Delegation viewing key for this (owner, spender) pair, pinned by
//           O3 to Poseidon2(delta_allow_r, dvk_i, sigma_a) = r_a.
//   v_a     Plaintext current allowance value.
//   r_a     Plaintext blinding factor for C_a (single-limb F_r; pinned by O3).
//   v_tx    Plaintext transfer amount.
//   r_e     Ephemeral scalar for recipient + auditor ECDH; must satisfy
//           r_e != 0 (O13).

fn main(
    sk_op: Field,
    dvk_i: Field,
    v_a: Field,
    r_a: Field,
    v_tx: Field,
    r_e: Field,
    c_a_x: pub Field,
    c_a_y: pub Field,
    sigma_a: pub Field,
    y_op_x: pub Field,
    y_op_y: pub Field,
    pvk_recipient_x: pub Field,
    pvk_recipient_y: pub Field,
    k_aud_r_x: pub Field,
    k_aud_r_y: pub Field,
    k_aud_s_x: pub Field,
    k_aud_s_y: pub Field,
    c_a_new_x: pub Field,
    c_a_new_y: pub Field,
    c_tx_x: pub Field,
    c_tx_y: pub Field,
    r_e_x: pub Field,
    r_e_y: pub Field,
    v_tilde: pub Field,
    a_tilde_new: pub Field,
    sigma_a_new: pub Field,
    v_tilde_aud_r: pub Field,
    r_tilde_aud_r: pub Field,
    v_tilde_aud_s: pub Field,
    a_tilde_aud_s: pub Field,
) {
    // O13 -- runs first so the r_e = 0 attack is rejected before any
    // scalar mul against it could quietly produce the identity.
    assert(r_e != 0);

    // O1
    let y_op_derived = scalar_mul(sk_op, H);
    assert(y_op_derived.x == y_op_x);
    assert(y_op_derived.y == y_op_y);

    // O3 -- runs before O2 so the contract-binding check (r_a is pinned to
    // dvk_i, sigma_a) is in scope before O2 uses r_a as the blinding under
    // the Pedersen commitment.
    let r_a_derived = derive_allow_r(dvk_i, sigma_a);
    assert(r_a_derived == r_a);

    // O2
    let c_a_derived = commit(v_a, r_a);
    assert(c_a_derived.x == c_a_x);
    assert(c_a_derived.y == c_a_y);

    // O4 -- Section 2.6 spells out the 127-bit decomposition / recomposition
    // pattern; `assert_max_bit_size` is the Noir stdlib primitive that
    // implements it directly.
    v_a.assert_max_bit_size::<127>();
    v_tx.assert_max_bit_size::<127>();
    let v_a_new = v_a - v_tx;
    v_a_new.assert_max_bit_size::<127>();

    // O6
    let r_e_derived = scalar_mul(r_e, H);
    assert(r_e_derived.x == r_e_x);
    assert(r_e_derived.y == r_e_y);

    // O5 -- PVK_recipient is path (2) of Section 10.8 (proof-constrained at
    // the recipient's registration via R3, trusted on read).
    let pvk_recipient =
        EmbeddedCurvePoint { x: pvk_recipient_x, y: pvk_recipient_y, is_infinite: false };
    let s_x = ecdh(r_e, pvk_recipient);

    // O7 (anti-poisoning, Section 5.4)
    let r_tx = derive_tx_blind(s_x, sigma_a);

    // O8
    let c_tx_derived = commit(v_tx, r_tx);
    assert(c_tx_derived.x == c_tx_x);
    assert(c_tx_derived.y == c_tx_y);

    // O9
    let v_tilde_derived = encrypt_amount(v_tx, s_x, sigma_a);
    assert(v_tilde_derived == v_tilde);

    // Salt rotation (implementation hardening).
    assert(sigma_a_new != sigma_a);

    // O10
    let r_a_new = derive_allow_r(dvk_i, sigma_a_new);

    // O11
    let c_a_new_derived = commit(v_a_new, r_a_new);
    assert(c_a_new_derived.x == c_a_new_x);
    assert(c_a_new_derived.y == c_a_new_y);

    // O12
    let a_tilde_new_derived = encrypt_allowance(v_a_new, dvk_i, sigma_a_new);
    assert(a_tilde_new_derived == a_tilde_new);

    // K_aud_r / K_aud_s point validation (Section 10.8: public-input keys).
    let k_aud_r = EmbeddedCurvePoint { x: k_aud_r_x, y: k_aud_r_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_r);
    let k_aud_s = EmbeddedCurvePoint { x: k_aud_s_x, y: k_aud_s_y, is_infinite: false };
    assert_on_curve_non_identity(k_aud_s);

    // O_a1 (recipient-auditor shared secret x-coordinate)
    let s_a_r_x = ecdh(r_e, k_aud_r);

    // O_a2 (recipient-channel masks: amount, then r_tx)
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_a_r_x, sigma_a);

    // O_a3
    assert(v_tx + m_r[0] == v_tilde_aud_r);

    // O_a4
    assert(r_tx + m_r[1] == r_tilde_aud_r);

    // O_a5 (owner-auditor shared secret x-coordinate)
    let s_a_s_x = ecdh(r_e, k_aud_s);

    // O_a6 (owner-channel masks: amount, then post-transfer allowance)
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_a_s_x, sigma_a);

    // O_a7
    assert(v_tx + m_s[0] == v_tilde_aud_s);

    // O_a8
    assert(v_a_new + m_s[1] == a_tilde_aud_s);
}
