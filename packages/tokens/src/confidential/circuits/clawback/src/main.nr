use stellar_confidential_lib::commit;

mod tests;

// Clawback circuit -- COMPLIANCE.md Section 5.3.
//
// Proves that a compliance seizure of the public amount `alpha` is bounded by
// what the target actually holds, without revealing either balance. It is the
// only circuit in the system with no key-ownership constraint and no `r_e`:
// both openings are pinned by Pedersen binding directly, so there is nothing
// left to forge and nothing to pin.
//
// Constraints
// -----------
//   CB1 C_spend    = v_s * G + r_s * H       Prover knows the opening of the
//                                            target's spendable commitment.
//   CB2 C_receive  = v_r * G + r_r * H       Prover knows the opening of the
//                                            target's receiving commitment.
//   CB3 v_s, v_r, alpha, and
//       v_s + v_r - alpha in [0, 2^127)      The seize bound (Section 2.6).
//                                            The final term is the
//                                            soundness-critical one: it is
//                                            what makes
//                                            alpha <= v_s + v_r, and without
//                                            it an over-seize would drive the
//                                            committed value negative mod F_r
//                                            -- openable arithmetically but
//                                            never again satisfiable under
//                                            W4 / T4, i.e. an unspendable
//                                            account.
//
// Who can produce this witness
// ----------------------------
// Anyone holding both Pedersen openings: the auditor, which tracks them
// forward from the sender-auditor secret-escrow slot (W_a5 / T_a9 / S_a6), or
// the owner, which derives them from `vk`. **Not the compliance admin**, which
// holds no blinding. That asymmetry is the whole of COMPLIANCE.md Section
// 5.2's "neither party can act alone": the admin decides whether to seize, the
// witness-holder decides how much and where to. A colluding owner supplying
// the witness is harmless -- it is the party being seized from.
//
// Identity encoding
// -----------------
// The circuit MUST accept (0, 0) as the identity for either commitment, opened
// as (0, 0). `C_receive = O` whenever the target has nothing pending, which is
// the common case. `commit(0, 0)` returns the identity point, whose
// `EmbeddedCurvePoint` encoding has x = y = 0, so the equality checks below
// hold without a special case.
//
// Point-validation doctrine (Section 10.8)
// ----------------------------------------
// C_spend and C_receive are the only points, and both are bound to in-circuit
// multi_scalar_mul outputs (CB1, CB2), so they are on-curve by construction.
// There is no public-input key to validate: the circuit consumes no auditor
// key, no spending key, and no ephemeral.
//
// Public inputs (8 fields, in COMPLIANCE.md Section 5.3 canonical order)
// ----------------------------------------------------------------------
//   Idx  Param         Symbol      Source / Note
//   ---  -----         ------      ----------------------------------------
//   0    c_spend_x     C_spend.x   Loaded from the target's
//   1    c_spend_y     C_spend.y     `spendable_commitment`.
//   2    c_receive_x   C_receive.x Loaded from the target's
//   3    c_receive_y   C_receive.y   `receiving_commitment`.
//   4    alpha         alpha       Public seize amount, from the invocation.
//                                    The contract pre-checks alpha > 0 before
//                                    assembling the blob; CB3 closes
//                                    alpha < 2^127 in-circuit.
//   5    addr_f        addr_f      Loaded from instance storage.
//   6    _acct_f       acct_f      `address_to_field(account)`, computed
//                                    per-call.
//   7    _dest_f       dest_f      `address_to_field(destination)` under
//                                    `Some`, the zero field under `None`;
//                                    computed per-call.
//
// `addr_f`, `_acct_f`, and `_dest_f` are referenced by no gate and look like
// dead parameters. They are the bindings, on the `register/_acct_f` precedent:
// UltraHonk absorbs every public input into the transcript, so a proof
// produced against one contract, account, or settlement destination fails when
// the contract assembles the blob for another. Removing `_dest_f` in
// particular would turn a compromised clawback signer from a griefer into a
// thief -- it could take a witness the auditor built for one destination and
// settle it to an address of its own choosing. Do not prune them.
//
// The first two additionally cover the degenerate case of two accounts whose
// commitments are both O. Anti-replay comes from C_spend and C_receive
// themselves: both change on a successful seize, so a proof never verifies
// twice -- which holds only because the contract rejects alpha = 0, the sole
// state-preserving case (C_receive = O together with alpha = 0).
//
// Private witnesses
// -----------------
//   v_s   Plaintext spendable-balance value.
//   r_s   Plaintext blinding factor for C_spend.
//   v_r   Plaintext receiving-balance value.
//   r_r   Plaintext blinding factor for C_receive.

fn main(
    v_s: Field,
    r_s: Field,
    v_r: Field,
    r_r: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    c_receive_x: pub Field,
    c_receive_y: pub Field,
    alpha: pub Field,
    addr_f: pub Field,
    _acct_f: pub Field,
    _dest_f: pub Field,
) {
    // addr_f is a binding-only input like _acct_f and _dest_f; naming it
    // without a leading underscore would warn, so consume it explicitly.
    let _ = addr_f;

    // CB1
    let c_spend_derived = commit(v_s, r_s);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // CB2
    let c_receive_derived = commit(v_r, r_r);
    assert(c_receive_derived.x == c_receive_x);
    assert(c_receive_derived.y == c_receive_y);

    // CB3 -- Section 2.6's 127-bit decomposition / recomposition pattern.
    // The sum check is the soundness-critical step: v_s and v_r each in range
    // does not bound their difference from alpha.
    v_s.assert_max_bit_size::<127>();
    v_r.assert_max_bit_size::<127>();
    alpha.assert_max_bit_size::<127>();
    let v_new = v_s + v_r - alpha;
    v_new.assert_max_bit_size::<127>();
}
