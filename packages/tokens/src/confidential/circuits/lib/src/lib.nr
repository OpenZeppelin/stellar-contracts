//! # Confidential Token -- Noir Primitives
//!
//! Shared cryptographic primitives consumed by the six circuits described in
//! Section 7 of the design document (Register, Withdraw, Transfer, SpenderTransfer,
//! SetSpender, RevokeSpender).
//!
//! Every constraint that involves a Pedersen commitment, a Poseidon2 hash with
//! domain separation, an ECDH derivation, or a 128-bit range check funnels
//! through one of the helpers exported here. Circuits never hash raw --
//! `poseidon_with_domain` is the only Poseidon entry point.
//!
//! ## Background: BN254, Grumpkin, and why this works in-circuit
//!
//! An elliptic curve is defined over a *base field* and has a *scalar field*
//! of the same size as its prime-order subgroup. Noir circuits, when targeting
//! the Barretenberg backend (UltraHonk), operate natively over the **BN254
//! scalar field**, written `F_r` (Noir's built-in `Field` type *is* `F_r`).
//!
//! **Grumpkin** is a curve specifically constructed so that its *base field*
//! equals BN254's *scalar field*. That means Grumpkin point coordinates are
//! `F_r` elements -- the same field the circuit is already working over. So
//! Grumpkin point arithmetic inside a Noir circuit is native field
//! arithmetic. This is what makes ! Pedersen commitments and ECDH cheap in
//! this design.
//!
//! Grumpkin's equation is `y^2 = x^3 - 17`. For a longer write-up, see
//! Aztec's primer:
//! <https://aztecprotocol.github.io/aztec-connect/primitives.html#2-grumpkin---a-curve-on-top-of-bn-254-for-snark-efficient-group-operations>
//!
//! ## Generators
//!
//! Both generators below come from Barretenberg's standard Pedersen domain
//! ("DEFAULT_DOMAIN_SEPARATOR") via `std::hash::derive_generators`, so
//! `commit(v, r)` is point-for-point identical to
//! `std::hash::pedersen_commitment([v, r])`. The numeric values are extracted
//! via the `print_generators` test (run
//! `nargo test print_generators --show-output`) so they are reproducible by
//! anyone with a Noir toolchain -- no off-chain trust required.

mod tests;

use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, multi_scalar_mul};
use std::hash::poseidon2_permutation;

// ################## GENERATORS ##################

/// Barretenberg's Pedersen generator at index 0 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[0]`.
pub global G: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x083e7911d835097629f0067531fc15cafd79a89beecb39903f69572c636f4a5a,
    y: 0x1a7f5efaad7f315c25a918f30cc8d7333fccab7ad7c90f14de81bcc528f9935d,
    is_infinite: false,
};

/// Barretenberg's Pedersen generator at index 1 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[1]`.
/// No known discrete-log relation to G (each is the output of Barretenberg's
/// hash-to-curve on the domain separator + index).
pub global H: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x054aa86a73cb8a34525e5bbed6e43ba1198e860f5f3950268f71df4591bde402,
    y: 0x209dcfbf2cfb57f9f6046f44d71ac6faf87254afc7407c04eb621a6287cac126,
    is_infinite: false,
};

// ################## CONSTANTS ##################

/// Poseidon2 sponge Initialization Vector (IV) multiplier:
/// `iv = (input_length as Field) * 2^64`. Matches Noir stdlib's
/// `Poseidon2Hasher` and Barretenberg's Poseidon2 sponge.
global POSEIDON2_IV_BASE: Field = 18446744073709551616; // 2^64

/// Domain separation tags (Section 13 of the design document). The integer
/// values are the cross-language contract -- they appear as the first absorbed
/// element in every Poseidon2 call and the SDK must use the exact same numbers.
/// The names spell out what the design doc abbreviates; the table below maps
/// design-doc notation to the constant defined here.
///
/// | Design doc        | Value | Constant                          |
/// |:------------------|:------|:----------------------------------|
/// | `delta_addr`      | 1     | `ADDRESS`                         |
/// | `delta_vk`        | 2     | `VIEWING_KEY`                     |
/// | `delta_dvk`       | 3     | `DELEGATION_VIEWING_KEY`          |
/// | `delta_spend_r`   | 4     | `SPEND_RANDOMNESS`                |
/// | `delta_tx_blind`  | 5     | `TX_BLINDING`                     |
/// | `delta_tx_amount` | 6     | `TX_AMOUNT`                       |
/// | `delta_enc_bal`   | 7     | `ENCRYPTED_BALANCE`               |
/// | `delta_enc_allow` | 8     | `ENCRYPTED_ALLOWANCE`             |
/// | `delta_allow_r`   | 9     | `ALLOWANCE_RANDOMNESS`            |
/// | `delta_esc_dvk`   | 10    | `ESCROWED_DELEGATION_VIEWING_KEY` |
/// | `delta_aud_s`     | 11    | `AUDITOR_SENDER`                  |
/// | `delta_aud_r`     | 12    | `AUDITOR_RECIPIENT`               |
pub mod domain {
    /// Soroban Address compression into a single `F_r` Field:
    /// `address_to_field(a) = Poseidon2(ADDRESS, lo(a), hi(a))`.
    /// Section 2.7 (`delta_addr`). The contract computes `addr_f` and `op_i`
    /// off-circuit via this tag; circuits consume the resulting Field directly.
    pub global ADDRESS: Field = 1;
    /// Viewing-key derivation: `vk = Poseidon2(VIEWING_KEY, sk, addr_f)`.
    /// Section 4.2 (`delta_vk`).
    pub global VIEWING_KEY: Field = 2;
    /// Delegation viewing-key derivation:
    /// `dvk = Poseidon2(DELEGATION_VIEWING_KEY, vk, op_i)`. Section 4.4 (`delta_dvk`).
    pub global DELEGATION_VIEWING_KEY: Field = 3;
    /// Deterministic randomness for the new spendable balance:
    /// `r' = Poseidon2(SPEND_RANDOMNESS, vk, sigma)`. Constraints W5/T10/S9/V6 (`delta_spend_r`).
    pub global SPEND_RANDOMNESS: Field = 4;
    /// ECDH-derived blinding for the transfer commitment:
    /// `r_tx = Poseidon2(TX_BLINDING, s, sigma)`. Constraints T7/O7 (`delta_tx_blind`).
    pub global TX_BLINDING: Field = 5;
    /// Mask for the encrypted transfer amount:
    /// `v_tilde = v_tx + Poseidon2(TX_AMOUNT, s, sigma)`. Constraints T9/O9 (`delta_tx_amount`).
    pub global TX_AMOUNT: Field = 6;
    /// Mask for the encrypted balance scalar:
    /// `b_tilde = v_new + Poseidon2(ENCRYPTED_BALANCE, vk, sigma)`.
    /// Constraints W7/T12/S11/V8 (`delta_enc_bal`).
    pub global ENCRYPTED_BALANCE: Field = 7;
    /// Mask for the encrypted allowance scalar:
    /// `a_tilde = v_a + Poseidon2(ENCRYPTED_ALLOWANCE, dvk, sigma_a)`.
    /// Constraints S8/O12 (`delta_enc_allow`).
    pub global ENCRYPTED_ALLOWANCE: Field = 8;
    /// Deterministic randomness for the spender allowance:
    /// `r_a = Poseidon2(ALLOWANCE_RANDOMNESS, dvk, sigma_a)`.
    /// Constraints S6/O3/O10 (`delta_allow_r`).
    pub global ALLOWANCE_RANDOMNESS: Field = 9;
    /// Delegation-key escrow mask (spender ECDH):
    /// `Poseidon2(ESCROWED_DELEGATION_VIEWING_KEY, s, op_i)`.
    /// Constraint S12, Section 7.11 (`delta_esc_dvk`).
    pub global ESCROWED_DELEGATION_VIEWING_KEY: Field = 10;
    /// Sender or owner-auditor channel tag for Poseidon2 sponge masks
    /// (Section 2.5, Section 8.1). Squeeze 1 yields the amount mask (where
    /// applicable); squeeze 2 yields the balance/allowance checkpoint mask.
    /// Constraints W_a3 / T_a6 / S_a3 / V_a3 / O_a6 (`delta_aud_s`).
    pub global AUDITOR_SENDER: Field = 11;
    /// Recipient-auditor channel tag for Poseidon2 sponge masks
    /// (Section 2.5, Section 8.1). Squeeze 1 yields the amount mask; squeeze
    /// 2 yields the per-transfer Pedersen randomness mask.
    /// Constraints T_a2 / O_a2 (`delta_aud_r`).
    pub global AUDITOR_RECIPIENT: Field = 12;
}

// ################## CORE PRIMITIVES ##################

/// Pedersen commitment `v*G + r*H` on Grumpkin. Section 2.3.
pub fn commit(value: Field, randomness: Field) -> EmbeddedCurvePoint {
    multi_scalar_mul(
        [G, H],
        [EmbeddedCurveScalar::from_field(value), EmbeddedCurveScalar::from_field(randomness)],
    )
}

/// Scalar multiplication `s*P` on Grumpkin. Used for `Y = sk*H`, `PVK = vk*H`,
/// `R_e = r_e*H`, and as the building block under `ecdh`.
pub fn scalar_mul(scalar: Field, point: EmbeddedCurvePoint) -> EmbeddedCurvePoint {
    multi_scalar_mul([point], [EmbeddedCurveScalar::from_field(scalar)])
}

/// ECDH shared-secret extraction: returns `(s*P).x` per Section 2.4.
///
/// Use this whenever the *scalar* shared secret is what downstream Poseidon
/// helpers expect (every `derive_tx_blind` / `encrypt_amount` /
/// `encrypt_auditor_*` call). When the full shared-secret *point* is needed,
/// reach for `scalar_mul` directly.
pub fn ecdh(scalar: Field, point: EmbeddedCurvePoint) -> Field {
    scalar_mul(scalar, point).x
}

/// Asserts `p` is on Grumpkin AND not the identity. Use this for every
/// public-input *key* (Section 7.1 point 4: `R_e`, `Y`, `PVK`, `K_aud` must all
/// satisfy `P != O`). The identity element collapses ECDH (`S = O` regardless
/// of the secret scalar), so accepting it would let an adversary publish a key
/// for which every shared secret is known.
pub fn assert_on_curve_non_identity(p: EmbeddedCurvePoint) {
    assert(!p.is_infinite);
    assert(p.y * p.y == p.x * p.x * p.x - 17);
}

// ################## POSEIDON2 FUNNEL ##################

/// Poseidon2 sponge with rate=3, capacity=1, matching Noir stdlib's hasher and
/// Barretenberg's Poseidon2 sponge. IV = `len * 2^64` placed at the capacity
/// position; absorb `RATE` field elements per permutation; squeeze `state[0]`.
fn sponge<let M: u32>(inputs: [Field; M]) -> Field {
    let iv: Field = (M as Field) * POSEIDON2_IV_BASE;
    let mut state: [Field; 4] = [0, 0, 0, iv];

    let full: u32 = M / 3;
    for i in 0..full {
        state[0] += inputs[i * 3];
        state[1] += inputs[i * 3 + 1];
        state[2] += inputs[i * 3 + 2];
        state = poseidon2_permutation(state, 4);
    }

    let remainder: u32 = M - full * 3;
    if remainder != 0 {
        for i in 0..remainder {
            state[i] += inputs[full * 3 + i];
        }
        state = poseidon2_permutation(state, 4);
    }

    state[0]
}

/// Single Poseidon2 entry point. Every higher-level helper in this crate
/// (`vk_from_sk`, `dvk_from_vk_op`, all `derive_*` / `encrypt_*`) routes through
/// this funnel so that the domain tag is always the *first* absorbed element.
///
/// Hashing raw -- bypassing this -- is a violation of the library contract.
pub fn poseidon_with_domain<let N: u32>(d: Field, inputs: [Field; N]) -> Field {
    let mut buf: [Field; N + 1] = [0; N + 1];
    buf[0] = d;
    for i in 0..N {
        buf[i + 1] = inputs[i];
    }
    sponge(buf)
}

// ################## KEY DERIVATION ##################

/// Viewing key. Section 4.2. `vk = Poseidon2(VK, sk, addr_f)`.
///
/// The `addr_f` parameter (the contract contract's address as a `Field`) binds the
/// viewing key -- and therefore every proof that constrains `vk` (R2/W2/T2/S2/V2)
/// -- to a specific deployed contract. Dropping `addr_f` here would let a proof for
/// one contract verify against another's verifier.
pub fn vk_from_sk(sk: Field, addr_f: Field) -> Field {
    poseidon_with_domain(domain::VIEWING_KEY, [sk, addr_f])
}

/// Public viewing key `PVK = vk*H`. Section 4.3. The recipient's ECDH long-term key
/// for incoming transfers.
pub fn pvk_from_vk(vk: Field) -> EmbeddedCurvePoint {
    scalar_mul(vk, H)
}

/// Delegation viewing key. Section 4.4. `dvk_i = Poseidon2(DVK, vk, op_i)`.
pub fn dvk_from_vk_op(vk: Field, op_i: Field) -> Field {
    poseidon_with_domain(domain::DELEGATION_VIEWING_KEY, [vk, op_i])
}

/// Deterministic randomness for the new spendable balance after an
/// owner-initiated proof operation. Constraints W5 / T10 / S9 / V6.
pub fn derive_spend_r(vk: Field, sigma: Field) -> Field {
    poseidon_with_domain(domain::SPEND_RANDOMNESS, [vk, sigma])
}

/// Deterministic randomness for the spender allowance commitment.
/// Constraints S6 / O3 / O10.
pub fn derive_allow_r(dvk: Field, sigma_a: Field) -> Field {
    poseidon_with_domain(domain::ALLOWANCE_RANDOMNESS, [dvk, sigma_a])
}

/// ECDH-derived blinding for the transfer commitment `C_tx`.
/// Constraints T7 / O7. `s` is the recipient-ECDH shared secret x-coordinate.
pub fn derive_tx_blind(s: Field, sigma: Field) -> Field {
    poseidon_with_domain(domain::TX_BLINDING, [s, sigma])
}

// ################## ENCRYPTED SCALARS ##################

/// Encrypts the transfer amount under the recipient's ECDH shared secret.
/// Constraints T9 / O9. `v_tilde = v_tx + Poseidon2(TX_AMOUNT, s, sigma)`.
pub fn encrypt_amount(v_tx: Field, s: Field, sigma: Field) -> Field {
    v_tx + poseidon_with_domain(domain::TX_AMOUNT, [s, sigma])
}

/// Encrypts the new spendable balance under the owner's viewing key.
/// Constraints W7 / T12 / S11 / V8. `b_tilde = v_new + Poseidon2(ENC_BAL, vk, sigma)`.
pub fn encrypt_balance(v_new: Field, vk: Field, sigma: Field) -> Field {
    v_new + poseidon_with_domain(domain::ENCRYPTED_BALANCE, [vk, sigma])
}

/// Encrypts the spender allowance under the delegation viewing key.
/// Constraints S8 / O12. `a_tilde = v_a + Poseidon2(ENC_ALLOW, dvk, sigma_a)`.
pub fn encrypt_allowance(v_a: Field, dvk: Field, sigma_a: Field) -> Field {
    v_a + poseidon_with_domain(domain::ENCRYPTED_ALLOWANCE, [dvk, sigma_a])
}

/// Delegation-key escrow mask for the spender handoff at `set_spender`. Section 7.11.
/// `escrowed_dvk = dvk + Poseidon2(ESC_DVK, s, op_i)` where `s = (r_e * Y_op).x`.
pub fn encrypt_esc_dvk(dvk: Field, s: Field, op_i: Field) -> Field {
    dvk + poseidon_with_domain(domain::ESCROWED_DELEGATION_VIEWING_KEY, [s, op_i])
}

// ################## AUDITOR-CHANNEL SPONGE ##################

/// Poseidon2 sponge with two squeezes. Absorbs `(d, s_x, sigma)` and returns
/// the first two rate-elements of the post-permutation state in order.
///
/// Implements `SpongeSqueeze_2(delta, S.x, sigma)` from Section 2.5. The canonical
/// squeeze order is: index 0 = amount mask, index 1 = balance / randomness
/// mask, fixed by Sections 7 and 8. With rate = 3 the absorb fits one block
/// and both outputs are served from the same permutation, so the constraint
/// cost is exactly one Poseidon2 permutation per call.
///
/// Used by: Transfer (T_a2 recipient-auditor amount + randomness, T_a6
/// sender-auditor amount + balance) and SpenderTransfer (O_a2, O_a6).
pub fn sponge_squeeze_2(d: Field, s_x: Field, sigma: Field) -> [Field; 2] {
    let iv: Field = 3 * POSEIDON2_IV_BASE;
    let state = poseidon2_permutation([d, s_x, sigma, iv], 4);
    [state[0], state[1]]
}

/// Sender-auditor encrypted balance/allowance checkpoint for circuits whose
/// auditor block emits a single ciphertext (Withdraw W_a3/W_a4, RevokeSpender
/// V_a3/V_a5, SetSpender S_a3/S_a5).
///
/// `b_tilde_aud_s = v_new + Poseidon2(AUDITOR_SENDER, s_a_s_x, sigma)`.
/// The two-squeeze sender-channel form (Transfer T_a6-T_a8, SpenderTransfer
/// O_a6-O_a8) is a different composition that callers spell out directly with
/// `sponge_squeeze_2`.
pub fn encrypt_auditor_sender_balance(v_new: Field, s_a_s_x: Field, sigma: Field) -> Field {
    v_new + poseidon_with_domain(domain::AUDITOR_SENDER, [s_a_s_x, sigma])
}
