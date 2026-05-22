use stellar_confidential_lib::{
    H, assert_range_128, commit, derive_spend_r, encrypt_balance, scalar_mul, vk_from_sk,
};

mod tests;

// Withdraw circuit -- design doc Section 7.5.
//
// Constraints (main block W1-W7; auditor block W_a1-W_a5 follows)
// ---------------------------------------------------------------
//   W1 Y         = sk * H                              owner key ownership
//   W2 vk        = Poseidon2(delta_vk, sk, wrap)       wrapper-bound viewing
//                                                      key
//   W3 C_spend   = v * G + r * H                       opening of current
//                                                      spendable balance
//   W4 v, a, v - a in [0, 2^128)                       range validity
//                                                      (Section 2.6)
//   W5 r'        = Poseidon2(delta_spend_r, vk, sigma) deterministic
//                                                      randomness for the
//                                                      new balance
//   W6 C_spend'  = (v - a) * G + r' * H                new spendable
//                                                      commitment
//   W7 b_tilde   = (v - a)
//                    + Poseidon2(delta_enc_bal, vk, sigma)
//                                                      encrypted balance
//                                                      scalar (emitted)
//
// On-curve checks for Y, C_spend, and C_spend' are not needed: all three
// are bound to in-circuit multi_scalar_mul outputs (Y in W1, C_spend in W3,
// C_spend' in W6), which are on-curve by construction. See design doc
// Section 10.8 for the system-wide point-validation doctrine.
//
// Public inputs (this block: 10 fields; the auditor block will add 5 more
// in design-doc canonical order before this is shipped end-to-end)
// ---------------------------------------------------------------------
//   Idx  Param          Symbol      Source / Note
//   ---  -----          ------      ---------------------------------------
//   0    c_spend_x      C_spend.x   Loaded from `from.spendable_balance`.
//   1    c_spend_y      C_spend.y
//   2    y_x            Y.x         Loaded from `from.spending_key`.
//   3    y_y            Y.y
//   4    wrap           wrap        `env.current_contract_address()`.
//   5    a              a           Public withdrawal amount; the wrapper
//                                   pre-checks a >= 0 at the entrypoint,
//                                   W4 then closes a < 2^128 in-circuit.
//   6    c_spend_new_x  C_spend'.x  Prover-supplied; written to
//   7    c_spend_new_y  C_spend'.y    `from.spendable_balance` on success.
//   8    sigma          sigma       Prover-supplied random salt; emitted.
//   9    b_tilde        b_tilde     Prover-supplied encrypted balance
//                                   scalar; emitted.
//
// Private witnesses
// -----------------
//   sk   Spending secret scalar.
//   v    Plaintext spendable-balance value.
//   r    Plaintext blinding factor for C_spend.

fn main(
    sk: Field,
    v: Field,
    r: Field,
    c_spend_x: pub Field,
    c_spend_y: pub Field,
    y_x: pub Field,
    y_y: pub Field,
    wrap: pub Field,
    a: pub Field,
    c_spend_new_x: pub Field,
    c_spend_new_y: pub Field,
    sigma: pub Field,
    b_tilde: pub Field,
) {
    // W1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // W2
    let vk = vk_from_sk(sk, wrap);

    // W3
    let c_spend_derived = commit(v, r);
    assert(c_spend_derived.x == c_spend_x);
    assert(c_spend_derived.y == c_spend_y);

    // W4
    assert_range_128(v);
    assert_range_128(a);
    let v_new = v - a;
    assert_range_128(v_new);

    // W5
    let r_new = derive_spend_r(vk, sigma);

    // W6
    let c_spend_new_derived = commit(v_new, r_new);
    assert(c_spend_new_derived.x == c_spend_new_x);
    assert(c_spend_new_derived.y == c_spend_new_y);

    // W7
    let b_tilde_derived = encrypt_balance(v_new, vk, sigma);
    assert(b_tilde_derived == b_tilde);
}
