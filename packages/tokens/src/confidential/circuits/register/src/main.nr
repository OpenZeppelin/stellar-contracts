use stellar_confidential_lib::{H, pvk_from_vk, scalar_mul, vk_from_sk};

mod tests;

// Register circuit -- design doc Section 7.2.
//
// Constraints
// -----------
//   R1   Y   = sk * H                          spending key well-formed
//   R2   vk  = Poseidon2(delta_vk, sk, addr_f)   contract-bound viewing key
//   R3   PVK = vk * H                          public viewing key matches vk
//   R4   sk != 0                               rules out Y = O
//   R5   vk != 0                               rules out PVK = O (would
//                                              collapse every incoming
//                                              transfer's ECDH)
//
// On-curve checks for Y and PVK are not needed: both are bound to in-circuit
// `scalar_mul` outputs, which are on-curve by construction (Noir's embedded-
// curve ops cannot produce an off-curve Grumpkin point).See design doc
// Section 10.8 for the system-wide point-validation doctrine.
//
// Public inputs
// -------------
// `Y` and `PVK` each occupy two Field slots (Grumpkin affine coordinates).
//
//   Idx  Param  Symbol  Source / Note
//   ---  -----  ------  -----------------------------------------------
//   0    y_x    Y.x     Prover-supplied spending public key. Written
//   1    y_y    Y.y       to `account.spending_key` on success.
//   2    pvk_x  PVK.x   Prover-supplied public viewing key. Written
//   3    pvk_y  PVK.y     to `account.viewing_public_key` on success.
//   4    addr_f   addr_f    `env.current_contract_address()` - binds the
//                         proof to a specific contract instance.
//
// Private witness
// ---------------
//   sk           Spending secret scalar; never revealed by the proof.
fn main(
    sk: Field,
    y_x: pub Field,
    y_y: pub Field,
    pvk_x: pub Field,
    pvk_y: pub Field,
    addr_f: pub Field,
) {
    // R4
    assert(sk != 0);

    // R1
    let y_derived = scalar_mul(sk, H);
    assert(y_derived.x == y_x);
    assert(y_derived.y == y_y);

    // R2
    let vk = vk_from_sk(sk, addr_f);

    // R5
    assert(vk != 0);

    // R3
    let pvk_derived = pvk_from_vk(vk);
    assert(pvk_derived.x == pvk_x);
    assert(pvk_derived.y == pvk_y);
}
