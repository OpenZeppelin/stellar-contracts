//! # Confidential Token Wrapper -- Noir Primitives
//!
//! Shared cryptographic primitives consumed by the six circuits described in
//! Section 7 of the design document (Register, Withdraw, Transfer, OperatorTransfer,
//! SetOperator, RevokeOperator).
//!
//! Every constraint that involves a Pedersen commitment, a Poseidon2 hash with
//! domain separation, an ECDH derivation, or a 128-bit range check funnels
//! through one of the helpers exported here. Circuits never hash raw --
//! `poseidon_with_domain` is the only Poseidon entry point.
//!
//! ## Curve
//!
//! Grumpkin `y^2 = x^3 - 17` over `Fr_{BN254}`. Both generators below come
//! from Barretenberg's standard Pedersen domain ("DEFAULT_DOMAIN_SEPARATOR")
//! via `std::hash::derive_generators`, so `commit(v, r)` is point-for-point
//! identical to `std::hash::pedersen_commitment([v, r])`. The numeric values
//! are extracted via the `print_generators` test (run `nargo test
//! print_generators --show-output`) so they are reproducible by anyone with
//! a Noir toolchain -- no off-chain trust required.

use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, multi_scalar_mul};
use std::hash::{derive_generators, poseidon2_permutation};

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

/// Poseidon2 sponge IV multiplier: `iv = (input_length as Field) * 2^64`.
/// Matches Noir stdlib's `Poseidon2Hasher` and Barretenberg's Poseidon2 sponge.
global POSEIDON2_IV_BASE: Field = 18446744073709551616; // 2^64

/// Domain separation tags (Section 13 of the design document). Module-private --
/// callers reach them through the typed `poseidon_with_domain` helpers below.
mod domain {
    /// Viewing-key derivation. `vk = Poseidon2(VK, sk, wrap)` -- Section 4.2.
    pub(crate) global VK: Field = 1;
    /// Delegation viewing-key derivation. `dvk = Poseidon2(DVK, vk, op_i)` -- Section 4.4.
    pub(crate) global DVK: Field = 2;
    /// Deterministic randomness for the spendable balance.
    /// `r' = Poseidon2(SPEND_R, vk, sigma)` -- constraints W5/T10/S9/V6.
    pub(crate) global SPEND_R: Field = 3;
    /// ECDH-derived blinding for the transfer commitment.
    /// `r_tx = Poseidon2(TX_BLIND, s, sigma)` -- constraints T7/O7.
    pub(crate) global TX_BLIND: Field = 4;
    /// ECDH-derived mask for the encrypted transfer amount.
    /// `v_tilde = v_tx + Poseidon2(TX_AMOUNT, s, sigma)` -- constraints T9/O9.
    pub(crate) global TX_AMOUNT: Field = 5;
    /// Mask for the encrypted balance scalar.
    /// `b_tilde = v_new + Poseidon2(ENC_BAL, vk, sigma)` -- constraints W7/T12/S11/V8.
    pub(crate) global ENC_BAL: Field = 6;
    /// Mask for the encrypted allowance scalar.
    /// `a_tilde = v_a + Poseidon2(ENC_ALLOW, dvk, sigma_a)` -- constraints S8/O12.
    pub(crate) global ENC_ALLOW: Field = 7;
    /// Deterministic randomness for the operator allowance.
    /// `r_a = Poseidon2(ALLOW_R, dvk, sigma_a)` -- constraints S6/O3/O10.
    pub(crate) global ALLOW_R: Field = 8;
    /// Delegation-key escrow (operator ECDH).
    /// `Poseidon2(ESC_DVK, s, op_i)` -- constraint S12 (Section 7.11).
    pub(crate) global ESC_DVK: Field = 9;
    /// Auditor mask for transfer amount.
    /// `v_tilde_aud = v_tx + Poseidon2(AUD_TX, s_a, sigma)` -- constraints
    /// T_a2 / T_a4 / O_a2 / O_a4 / S_a3 / V_a3.
    pub(crate) global AUD_TX: Field = 10;
    /// Auditor mask for balance/allowance checkpoint.
    /// `b_tilde_aud = v_new + Poseidon2(AUD_BAL, s_a, sigma)` -- constraints
    /// W_a3 / T_a5 / S_a4 / V_a4 / O_a5.
    pub(crate) global AUD_BAL: Field = 11;
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

/// Range check `x in [0, 2^128)` via Noir's native `u128` cast (Section 2.6). The
/// `as u128` lowers to 128 boolean constraints plus one recomposition check.
pub fn assert_range_128(x: Field) {
    let x_u128: u128 = x as u128;
    assert(x_u128 as Field == x);
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

/// Viewing key. Section 4.2. `vk = Poseidon2(VK, sk, wrap)`.
///
/// The `wrap` parameter (the wrapper contract's address as a `Field`) binds the
/// viewing key -- and therefore every proof that constrains `vk` (R2/W2/T2/S2/V2)
/// -- to a specific deployed wrapper. Dropping `wrap` here would let a proof for
/// one wrapper verify against another's verifier.
pub fn vk_from_sk(sk: Field, wrap: Field) -> Field {
    poseidon_with_domain(domain::VK, [sk, wrap])
}

/// Public viewing key `PVK = vk*H`. Section 4.3. The recipient's ECDH long-term key
/// for incoming transfers.
pub fn pvk_from_vk(vk: Field) -> EmbeddedCurvePoint {
    scalar_mul(vk, H)
}

/// Delegation viewing key. Section 4.4. `dvk_i = Poseidon2(DVK, vk, op_i)`.
pub fn dvk_from_vk_op(vk: Field, op_i: Field) -> Field {
    poseidon_with_domain(domain::DVK, [vk, op_i])
}

/// Deterministic randomness for the new spendable balance after an
/// owner-initiated proof operation. Constraints W5 / T10 / S9 / V6.
pub fn derive_spend_r(vk: Field, sigma: Field) -> Field {
    poseidon_with_domain(domain::SPEND_R, [vk, sigma])
}

/// Deterministic randomness for the operator allowance commitment.
/// Constraints S6 / O3 / O10.
pub fn derive_allow_r(dvk: Field, sigma_a: Field) -> Field {
    poseidon_with_domain(domain::ALLOW_R, [dvk, sigma_a])
}

/// ECDH-derived blinding for the transfer commitment `C_tx`.
/// Constraints T7 / O7. `s` is the recipient-ECDH shared secret x-coordinate.
pub fn derive_tx_blind(s: Field, sigma: Field) -> Field {
    poseidon_with_domain(domain::TX_BLIND, [s, sigma])
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
    v_new + poseidon_with_domain(domain::ENC_BAL, [vk, sigma])
}

/// Encrypts the operator allowance under the delegation viewing key.
/// Constraints S8 / O12. `a_tilde = v_a + Poseidon2(ENC_ALLOW, dvk, sigma_a)`.
pub fn encrypt_allowance(v_a: Field, dvk: Field, sigma_a: Field) -> Field {
    v_a + poseidon_with_domain(domain::ENC_ALLOW, [dvk, sigma_a])
}

/// Encrypts the transfer amount under an auditor's ECDH shared secret.
/// Constraints T_a2 / T_a4 / O_a2 / O_a4 / S_a3 / V_a3.
/// `v_tilde_aud = v_tx + Poseidon2(AUD_TX, s_a, sigma)`.
pub fn encrypt_auditor_tx(v_tx: Field, s_a: Field, sigma: Field) -> Field {
    v_tx + poseidon_with_domain(domain::AUD_TX, [s_a, sigma])
}

/// Encrypts the post-operation balance (or allowance) under an auditor's ECDH
/// shared secret. Constraints W_a3 / T_a5 / S_a4 / V_a4 / O_a5.
/// `b_tilde_aud = v_new + Poseidon2(AUD_BAL, s_a, sigma)`.
pub fn encrypt_auditor_bal(v_new: Field, s_a: Field, sigma: Field) -> Field {
    v_new + poseidon_with_domain(domain::AUD_BAL, [s_a, sigma])
}

/// Delegation-key escrow mask for the operator handoff at `set_operator`. Section 7.11.
/// `escrowed_dvk = dvk + Poseidon2(ESC_DVK, s, op_i)` where `s = (r_e * Y_op).x`.
pub fn encrypt_esc_dvk(dvk: Field, s: Field, op_i: Field) -> Field {
    dvk + poseidon_with_domain(domain::ESC_DVK, [s, op_i])
}

// ################## TESTS ##################

/// Provenance check: reproduces G and H from Barretenberg's `derive_generators`
/// at run time. If this test fails, the hardcoded constants above have drifted
/// from upstream and must be re-extracted. Run
/// `nargo test print_generators --show-output` to print the canonical values.
#[test]
fn print_generators() {
    let gens = derive_generators::<2, _>("DEFAULT_DOMAIN_SEPARATOR".as_bytes(), 0);
    let g_x = gens[0].x;
    let g_y = gens[0].y;
    let h_x = gens[1].x;
    let h_y = gens[1].y;
    println(f"G.x = {g_x}");
    println(f"G.y = {g_y}");
    println(f"H.x = {h_x}");
    println(f"H.y = {h_y}");
    assert(gens[0].x == G.x);
    assert(gens[0].y == G.y);
    assert(gens[0].is_infinite == G.is_infinite);
    assert(gens[1].x == H.x);
    assert(gens[1].y == H.y);
    assert(gens[1].is_infinite == H.is_infinite);
}

#[test]
fn commit_identity() {
    // Pedersen of (0, 0) is the curve identity, encoded as is_infinite = true.
    let o = commit(0, 0);
    assert(o.is_infinite);
}

#[test]
fn commit_deterministic() {
    let c1 = commit(100, 42);
    let c2 = commit(100, 42);
    assert(c1.x == c2.x);
    assert(c1.y == c2.y);
}

#[test]
fn commit_equals_pedersen_commitment() {
    // `commit(v, r)` is point-for-point identical to the stdlib's pedersen
    // commitment, since we use the same generators.
    let c = commit(123, 456);
    let p = std::hash::pedersen_commitment([123, 456]);
    assert(c.x == p.x);
    assert(c.y == p.y);
}

#[test]
fn commit_homomorphic() {
    // commit(v1, r1) + commit(v2, r2) == commit(v1+v2, r1+r2).
    let c1 = commit(100, 42);
    let c2 = commit(250, 7);
    let sum_a = c1 + c2;
    let sum_b = commit(350, 49);
    assert(sum_a.x == sum_b.x);
    assert(sum_a.y == sum_b.y);
}

#[test]
fn ecdh_symmetric() {
    // ECDH symmetry: (a) * (b*H) and (b) * (a*H) yield the same shared x.
    let a: Field = 12345;
    let b: Field = 67890;
    let a_pub = scalar_mul(a, H);
    let b_pub = scalar_mul(b, H);
    let s_ab = ecdh(a, b_pub);
    let s_ba = ecdh(b, a_pub);
    assert(s_ab == s_ba);
}

#[test]
fn range_128_boundaries() {
    assert_range_128(0);
    assert_range_128(1);
    assert_range_128(0xffffffffffffffffffffffffffffffff); // 2^128 - 1
}

#[test(should_fail)]
fn range_128_rejects_2_pow_128() {
    // 2^128 must NOT pass the 128-bit range check.
    let two_pow_128: Field = 0x100000000000000000000000000000000;
    assert_range_128(two_pow_128);
}

#[test]
fn vk_wrap_binding() {
    // vk_from_sk(sk, w1) != vk_from_sk(sk, w2). This is the property that
    // makes R2/W2/T2/S2/V2 wrapper-bound -- if it ever fails, a proof made
    // against one wrapper could verify against another.
    let sk: Field = 0xdeadbeef;
    let vk_a = vk_from_sk(sk, 0x111);
    let vk_b = vk_from_sk(sk, 0x222);
    assert(vk_a != vk_b);
}

#[test]
fn key_derivation_chain() {
    // sk -> vk -> PVK -> dvk all derive non-trivially and consistently.
    let sk: Field = 12345;
    let wrap: Field = 0xc0ffee;
    let vk = vk_from_sk(sk, wrap);
    let pvk = pvk_from_vk(vk);
    let y = scalar_mul(sk, H);
    let dvk = dvk_from_vk_op(vk, 0xabcd);

    // None collapse to identity.
    assert(!pvk.is_infinite);
    assert(!y.is_infinite);
    assert(dvk != 0);

    // Re-derivation is deterministic.
    assert(vk == vk_from_sk(sk, wrap));
    assert(dvk == dvk_from_vk_op(vk, 0xabcd));
}

#[test]
fn domain_separation_distinct() {
    // Same payload under two different domains must hash to different outputs.
    // Touches every domain tag at least once.
    let a: Field = 42;
    let b: Field = 99;
    let h_vk = poseidon_with_domain(domain::VK, [a, b]);
    let h_dvk = poseidon_with_domain(domain::DVK, [a, b]);
    let h_sr = poseidon_with_domain(domain::SPEND_R, [a, b]);
    let h_tb = poseidon_with_domain(domain::TX_BLIND, [a, b]);
    let h_ta = poseidon_with_domain(domain::TX_AMOUNT, [a, b]);
    let h_eb = poseidon_with_domain(domain::ENC_BAL, [a, b]);
    let h_ea = poseidon_with_domain(domain::ENC_ALLOW, [a, b]);
    let h_ar = poseidon_with_domain(domain::ALLOW_R, [a, b]);
    let h_ed = poseidon_with_domain(domain::ESC_DVK, [a, b]);
    let h_at = poseidon_with_domain(domain::AUD_TX, [a, b]);
    let h_ab = poseidon_with_domain(domain::AUD_BAL, [a, b]);

    // Distinct from VK.
    assert(h_vk != h_dvk);
    assert(h_vk != h_sr);
    assert(h_vk != h_tb);
    assert(h_vk != h_ta);
    assert(h_vk != h_eb);
    assert(h_vk != h_ea);
    assert(h_vk != h_ar);
    assert(h_vk != h_ed);
    assert(h_vk != h_at);
    assert(h_vk != h_ab);
}

#[test]
fn encrypt_decrypt_round_trip() {
    // Every `encrypt_*` helper must be decryptable by subtracting the same
    // mask. We re-derive the mask via poseidon_with_domain to confirm the
    // helpers are doing what they advertise.
    let v: Field = 1_000_000;
    let s: Field = 0xfeedface;
    let sigma: Field = 0xcafe;

    let v_tilde = encrypt_amount(v, s, sigma);
    let mask_tx = poseidon_with_domain(domain::TX_AMOUNT, [s, sigma]);
    assert(v_tilde - mask_tx == v);

    let b_tilde = encrypt_balance(v, s, sigma);
    let mask_bal = poseidon_with_domain(domain::ENC_BAL, [s, sigma]);
    assert(b_tilde - mask_bal == v);

    let a_tilde = encrypt_allowance(v, s, sigma);
    let mask_allow = poseidon_with_domain(domain::ENC_ALLOW, [s, sigma]);
    assert(a_tilde - mask_allow == v);

    let v_tilde_aud = encrypt_auditor_tx(v, s, sigma);
    let mask_aud_tx = poseidon_with_domain(domain::AUD_TX, [s, sigma]);
    assert(v_tilde_aud - mask_aud_tx == v);

    let b_tilde_aud = encrypt_auditor_bal(v, s, sigma);
    let mask_aud_bal = poseidon_with_domain(domain::AUD_BAL, [s, sigma]);
    assert(b_tilde_aud - mask_aud_bal == v);

    let dvk_escrowed = encrypt_esc_dvk(v, s, sigma);
    let mask_esc = poseidon_with_domain(domain::ESC_DVK, [s, sigma]);
    assert(dvk_escrowed - mask_esc == v);
}

/// One-shot fixture-generation harness. Prints every primitive's output for a
/// deterministic input set. Captured into `testdata/*.json` (the cross-language
/// contract for the SDK) and mirrored in `fixtures_match_testdata` below. Run
/// `nargo test print_fixtures --show-output` to regenerate.
#[test]
fn print_fixtures() {
    // Inputs (fixed). Any change here invalidates testdata/*.json.
    let sk: Field = 0xdead;
    let wrap: Field = 0xbeef;
    let sigma: Field = 0x01;
    let sigma_a: Field = 0x02;
    let op_i: Field = 0xabcd;
    let v: Field = 1000;
    let r: Field = 42;
    let v_tx: Field = 100;
    let v_a: Field = 500;
    let r_e: Field = 0xfeedface;
    let s_ecdh: Field = 0x12345;

    // commit(v, r)
    let c = commit(v, r);
    let cx = c.x;
    let cy = c.y;
    println(f"commit.x = {cx}");
    println(f"commit.y = {cy}");

    // scalar_mul(sk, H) -- Y
    let y_pt = scalar_mul(sk, H);
    let yx = y_pt.x;
    let yy = y_pt.y;
    println(f"scalar_mul_Y.x = {yx}");
    println(f"scalar_mul_Y.y = {yy}");

    // ecdh(r_e, H) -- shared secret x
    let e = ecdh(r_e, H);
    println(f"ecdh = {e}");

    // vk_from_sk(sk, wrap)
    let vk = vk_from_sk(sk, wrap);
    println(f"vk_from_sk = {vk}");

    // pvk_from_vk(vk)
    let pvk = pvk_from_vk(vk);
    let px = pvk.x;
    let py = pvk.y;
    println(f"pvk_from_vk.x = {px}");
    println(f"pvk_from_vk.y = {py}");

    // dvk_from_vk_op(vk, op_i)
    let dvk = dvk_from_vk_op(vk, op_i);
    println(f"dvk_from_vk_op = {dvk}");

    // derive_spend_r(vk, sigma)
    let r_prime = derive_spend_r(vk, sigma);
    println(f"derive_spend_r = {r_prime}");

    // derive_allow_r(dvk, sigma_a)
    let r_a = derive_allow_r(dvk, sigma_a);
    println(f"derive_allow_r = {r_a}");

    // derive_tx_blind(s, sigma)
    let r_tx = derive_tx_blind(s_ecdh, sigma);
    println(f"derive_tx_blind = {r_tx}");

    // encrypt_amount(v_tx, s, sigma)
    let v_tilde = encrypt_amount(v_tx, s_ecdh, sigma);
    println(f"encrypt_amount = {v_tilde}");

    // encrypt_balance(v, vk, sigma)
    let b_tilde = encrypt_balance(v, vk, sigma);
    println(f"encrypt_balance = {b_tilde}");

    // encrypt_allowance(v_a, dvk, sigma_a)
    let a_tilde = encrypt_allowance(v_a, dvk, sigma_a);
    println(f"encrypt_allowance = {a_tilde}");

    // encrypt_auditor_tx(v_tx, s, sigma)
    let v_aud = encrypt_auditor_tx(v_tx, s_ecdh, sigma);
    println(f"encrypt_auditor_tx = {v_aud}");

    // encrypt_auditor_bal(v, s, sigma)
    let b_aud = encrypt_auditor_bal(v, s_ecdh, sigma);
    println(f"encrypt_auditor_bal = {b_aud}");

    // encrypt_esc_dvk(dvk, s, op_i)
    let esc = encrypt_esc_dvk(dvk, s_ecdh, op_i);
    println(f"encrypt_esc_dvk = {esc}");

    // poseidon_with_domain(VK, [sk, wrap]) -- direct funnel access, equals vk_from_sk
    let pwd = poseidon_with_domain(domain::VK, [sk, wrap]);
    println(f"poseidon_with_domain_VK_2 = {pwd}");
}

/// Pins every primitive's output to the exact bytes published in
/// `testdata/*.json`. If this fails, either the fixtures need regenerating (via
/// `print_fixtures`) and the JSON files need updating to match -- both kept in
/// lockstep -- or a primitive's semantics drifted and we just broke every
/// downstream SDK consumer. Treat as a load-bearing test.
#[test]
fn fixtures_match_testdata() {
    let sk: Field = 0xdead;
    let wrap: Field = 0xbeef;
    let sigma: Field = 0x01;
    let sigma_a: Field = 0x02;
    let op_i: Field = 0xabcd;
    let v: Field = 1000;
    let r: Field = 42;
    let v_tx: Field = 100;
    let v_a: Field = 500;
    let r_e: Field = 0xfeedface;
    let s_ecdh: Field = 0x12345;

    let c = commit(v, r);
    assert(c.x == 0x195a5d8ecd032fe1696054b28f0852b5f03a613681991ace823245ab2f97ed05);
    assert(c.y == 0x0e67489ecfee0e581dce7db22a383c635886cfb616c3b3bb033bff0c46e9789e);

    let y_pt = scalar_mul(sk, H);
    assert(y_pt.x == 0x1b46b003b88a6c34549dc74115f088f4b231a151397526bc10cbf1d15b457646);
    assert(y_pt.y == 0x29116280600c10ead1fdbd9ab4b571896030679bf554d7f1ebf681e5147de21b);

    assert(
        ecdh(r_e, H) == 0x114ed4fcf2c57014eb678c577aa02f30ef590b713d7a6a5e87702d1c7f71957f,
    );

    let vk = vk_from_sk(sk, wrap);
    assert(vk == 0x0ac6ade91aea36f4de35eb5a06a67e1b9c3dab9842479f1cce85caf826194ec2);

    let pvk = pvk_from_vk(vk);
    assert(pvk.x == 0x01de20bfe21ad660e073b14a106feae4afce4e78a020953ca16b767e0615331b);
    assert(pvk.y == 0x12cf407b75fccb5c7721c5fdadc3d1a22b5440e36b08fe52658ab885529e92fd);

    let dvk = dvk_from_vk_op(vk, op_i);
    assert(dvk == 0x1dec1da326d325cf918eed33f097cfb854b41aac8ece9e6e8bd54b7a7c448785);

    assert(
        derive_spend_r(vk, sigma)
            == 0x1b9d813c7c14c78969edb37cce24f3bb867f6e64efe73d950cfbbe8ed4c31482,
    );
    assert(
        derive_allow_r(dvk, sigma_a)
            == 0x199b47ff8b24b165411fa44b06d541b9d1b74d0d202fd651712e24c11e05cb35,
    );
    assert(
        derive_tx_blind(s_ecdh, sigma)
            == 0x15388ad70c7a48cfed4fb9cfbd633ff18617c9e52829b5d51eaab938baae33a7,
    );

    assert(
        encrypt_amount(v_tx, s_ecdh, sigma)
            == 0x274c4f45d57544efca9f6f505e6b29ae098f1d30243de28f7cef62d05484c8a1,
    );
    assert(
        encrypt_balance(v, vk, sigma)
            == 0x2b7f92c72c05ae8ecf7207fdaecea32ef748cd4dd43ca9ed6dc6d226865b43af,
    );
    assert(
        encrypt_allowance(v_a, dvk, sigma_a)
            == 0x235db43a283e4e000337cdb80a99168ef4b718622cc06de4ab98b3d2bd5736d5,
    );
    assert(
        encrypt_auditor_tx(v_tx, s_ecdh, sigma)
            == 0x266e76dccd3d7c654ab7a48045cca920c497b1a6758981acfa6f217427ac30d8,
    );
    assert(
        encrypt_auditor_bal(v, s_ecdh, sigma)
            == 0x2787d1e01bbca7828e13e9b2b3fa11cfcfe9c3d2b121b9e17543146822fe23d3,
    );
    assert(
        encrypt_esc_dvk(dvk, s_ecdh, op_i)
            == 0x2468d02775ad21aa5928d049770287249af2a03dbaf4d18e22e3fe71fd292003,
    );

    // poseidon_with_domain funnel is the same operation that backs vk_from_sk.
    assert(poseidon_with_domain(domain::VK, [sk, wrap]) == vk);
}

#[test]
fn derive_helpers_deterministic_and_distinct() {
    // derive_spend_r, derive_allow_r, derive_tx_blind must each be
    // deterministic and produce distinct outputs from the same inputs
    // (domain separation hygiene).
    let a: Field = 0xbeef;
    let b: Field = 0xdead;

    let r1 = derive_spend_r(a, b);
    assert(r1 == derive_spend_r(a, b));

    let r2 = derive_allow_r(a, b);
    assert(r2 == derive_allow_r(a, b));

    let r3 = derive_tx_blind(a, b);
    assert(r3 == derive_tx_blind(a, b));

    assert(r1 != r2);
    assert(r1 != r3);
    assert(r2 != r3);
}
