use stellar_confidential_lib::{commit, scalar_mul, H};

mod tests;

// Clawback circuit -- docs/compliance.md#circuit.
//
// Proves that a compliance seizure of the public amount `alpha` is bounded by
// what the target actually holds, without revealing either balance, and that
// the prover is the auditor the target's account is bound to. It is the only
// circuit in the system with no `r_e`, and the only one whose key-ownership
// constraint is over an auditor key rather than an owner's.
//
// Constraints
// -----------
//   CB1 K_aud      = k_aud * H               Auditor key ownership. The
//                                            openings alone are not a secret:
//                                            registration, deposits, and
//                                            merges leave them public, so
//                                            without CB1 a clawback signer
//                                            could prove a seizure of any
//                                            such account by itself.
//   CB2 C_spend    = v_s * G + r_s * H       Prover knows the opening of the
//                                            target's spendable commitment.
//   CB3 C_receive  = v_r * G + r_r * H       Prover knows the opening of the
//                                            target's receiving commitment.
//   CB4 v_s, v_r, alpha, and
//       v_s + v_r - alpha in [0, 2^127)      The seize bound (docs/protocol/primitives.md#integer-embedding-and-range-proofs).
//                                            The final term establishes
//                                            alpha <= v_s + v_r.
//
// Identity encoding
// -----------------
// The circuit MUST accept (0, 0) as the identity for either commitment, opened
// as (0, 0). `C_receive = O` whenever the target has nothing pending.
// `commit(0, 0)` returns the identity point, whose `EmbeddedCurvePoint`
// encoding has x = y = 0, so the equality checks below hold without a special
// case.
//
// Point-validation
// ----------------------------------------
// K_aud, C_spend and C_receive are the only points, and all three are bound to
// in-circuit multi_scalar_mul outputs (CB1, CB2, CB3), so they are on-curve by
// construction.
//
// Public inputs (11 fields, in docs/compliance.md#circuit canonical order)
// -----------------------------------------------------------------------
//   Idx  Param         Symbol      Source / Note
//   ---  -----         ------      ----------------------------------------
//   0    c_spend_x     C_spend.x   Loaded from the target's
//   1    c_spend_y     C_spend.y     `spendable_commitment`.
//   2    c_receive_x   C_receive.x Loaded from the target's
//   3    c_receive_y   C_receive.y   `receiving_commitment`.
//   4    k_aud_x       K_aud.x     Fetched from the auditor registry under
//   5    k_aud_y       K_aud.y       the target's `auditor_id`.
//   6    alpha         alpha       Public seize amount, from the invocation.
//                                    The contract pre-checks alpha > 0 before
//                                    assembling the blob; CB4 closes
//                                    alpha < 2^127 in-circuit.
//   7    addr_f        addr_f      Loaded from instance storage.
//   8    _acct_f       acct_f      `address_to_field(account)`, computed
//                                    per-call.
//   9    _dest_f       dest_f      `address_to_field(destination)` under
//                                    `Some`, the zero field under `None`;
//                                    computed per-call.
//   10   _nonce        nonce       The target's clawback nonce, loaded from
//                                    storage; the contract increments it on
//                                    every seizure (docs/compliance.md#contract-flow).
//
// `addr_f`, `_acct_f`, `_dest_f`, and `_nonce` are referenced by no gate and
// look like dead parameters: their presence in the public-input set is the
// binding, on the `register/_acct_f` precedent. `_nonce` is what makes a proof
// single-use: the commitments alone can be restored to an earlier value
// (docs/compliance.md#anti-replay-and-the-freeze).
//
// Private witnesses
// -----------------
//   k_aud Auditor secret scalar for the target's `auditor_id`.
//   v_s   Plaintext spendable-balance value.
//   r_s   Plaintext blinding factor for C_spend.
//   v_r   Plaintext receiving-balance value.
//   r_r   Plaintext blinding factor for C_receive.

fn main(
    k_aud: Field,
    v_s: Field,
    r_s: Field,
    v_r: Field,
    r_r: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    c_receive_x: pub Field,
    c_receive_y: pub Field,
    k_aud_x: pub Field,
    k_aud_y: pub Field,
    alpha: pub Field,
    addr_f: pub Field,
    _acct_f: pub Field,
    _dest_f: pub Field,
    _nonce: pub Field,
) {
    // addr_f is a binding-only input like _acct_f, _dest_f, and _nonce; naming
    // it without a leading underscore would warn, so consume it explicitly.
    let _ = addr_f;

    // CB1
    let k_aud_derived = scalar_mul(k_aud, H);
    assert(k_aud_derived.x == k_aud_x);
    assert(k_aud_derived.y == k_aud_y);

    // CB2
    let c_spend_derived = commit(v_s, r_s);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // CB3
    let c_receive_derived = commit(v_r, r_r);
    assert(c_receive_derived.x == c_receive_x);
    assert(c_receive_derived.y == c_receive_y);

    // CB4 -- docs/protocol/primitives.md#integer-embedding-and-range-proofs's 127-bit decomposition / recomposition pattern.
    // Range on v_s and v_r alone does not bound v_s + v_r - alpha.
    v_s.assert_max_bit_size::<127>();
    v_r.assert_max_bit_size::<127>();
    alpha.assert_max_bit_size::<127>();
    let v_new = v_s + v_r - alpha;
    v_new.assert_max_bit_size::<127>();
}
