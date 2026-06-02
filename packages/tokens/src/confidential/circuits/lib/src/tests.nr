use crate::{
    assert_on_curve_non_identity, commit, derive_allow_r, derive_spend_r, derive_tx_blind, domain,
    dvk_from_vk_op, ecdh, encrypt_allowance, encrypt_amount, encrypt_auditor_sender_balance,
    encrypt_balance, encrypt_esc_dvk, G, H, poseidon_with_domain, pvk_from_vk, scalar_mul,
    sponge_squeeze_2, vk_from_sk,
};
use std::embedded_curve_ops::EmbeddedCurvePoint;
use std::hash::{derive_generators, pedersen_commitment};

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
    let p = pedersen_commitment([123, 456]);
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
    let s_a_b = ecdh(a, b_pub);
    let s_b_a = ecdh(b, a_pub);
    assert(s_a_b == s_b_a);
}

#[test]
fn on_curve_accepts_G_and_H() {
    // Both generators are valid Grumpkin points (no DL relation).
    assert_on_curve_non_identity(G);
    assert_on_curve_non_identity(H);
}

#[test(should_fail)]
fn on_curve_non_identity_rejects_identity() {
    // Identity must be rejected when the point will be used as a key in ECDH:
    // S = sk * O = O regardless of sk, collapsing the shared secret.
    let o = EmbeddedCurvePoint { x: 0, y: 0, is_infinite: true };
    assert_on_curve_non_identity(o);
}

#[test(should_fail)]
fn on_curve_non_identity_rejects_off_curve() {
    let bad = EmbeddedCurvePoint { x: 7, y: 11, is_infinite: false };
    assert_on_curve_non_identity(bad);
}

#[test]
fn vk_addr_f_binding() {
    // vk_from_sk(sk, w1) != vk_from_sk(sk, w2). This is the property that
    // makes R2/W2/T2/S2/V2 contract-bound -- if it ever fails, a proof made
    // against one contract could verify against another.
    let sk: Field = 0xdeadbeef;
    let vk_a = vk_from_sk(sk, 0x111);
    let vk_b = vk_from_sk(sk, 0x222);
    assert(vk_a != vk_b);
}

#[test]
fn key_derivation_chain() {
    // sk -> vk -> PVK -> dvk all derive non-trivially and consistently.
    let sk: Field = 12345;
    let addr_f: Field = 0xc0ffee;
    let vk = vk_from_sk(sk, addr_f);
    let pvk = pvk_from_vk(vk);
    let y = scalar_mul(sk, H);
    let dvk = dvk_from_vk_op(vk, 0xabcd);

    // None collapse to identity.
    assert(!pvk.is_infinite);
    assert(!y.is_infinite);
    assert(dvk != 0);

    // Re-derivation is deterministic.
    assert(vk == vk_from_sk(sk, addr_f));
    assert(dvk == dvk_from_vk_op(vk, 0xabcd));
}

#[test]
fn domain_separation_distinct() {
    // Same payload under two different domains must hash to different outputs.
    // Touches every domain tag at least once.
    let a: Field = 42;
    let b: Field = 99;
    let h_addr = poseidon_with_domain(domain::ADDRESS, [a, b]);
    let h_vk = poseidon_with_domain(domain::VIEWING_KEY, [a, b]);
    let h_dvk = poseidon_with_domain(domain::DELEGATION_VIEWING_KEY, [a, b]);
    let h_sr = poseidon_with_domain(domain::SPEND_RANDOMNESS, [a, b]);
    let h_tb = poseidon_with_domain(domain::TX_BLINDING, [a, b]);
    let h_ta = poseidon_with_domain(domain::TX_AMOUNT, [a, b]);
    let h_eb = poseidon_with_domain(domain::ENCRYPTED_BALANCE, [a, b]);
    let h_ea = poseidon_with_domain(domain::ENCRYPTED_ALLOWANCE, [a, b]);
    let h_ar = poseidon_with_domain(domain::ALLOWANCE_RANDOMNESS, [a, b]);
    let h_ed = poseidon_with_domain(domain::ESCROWED_DELEGATION_VIEWING_KEY, [a, b]);
    let h_as = poseidon_with_domain(domain::AUDITOR_SENDER, [a, b]);
    let h_ar2 = poseidon_with_domain(domain::AUDITOR_RECIPIENT, [a, b]);

    // Distinct from VK.
    assert(h_vk != h_addr);
    assert(h_vk != h_dvk);
    assert(h_vk != h_sr);
    assert(h_vk != h_tb);
    assert(h_vk != h_ta);
    assert(h_vk != h_eb);
    assert(h_vk != h_ea);
    assert(h_vk != h_ar);
    assert(h_vk != h_ed);
    assert(h_vk != h_as);
    assert(h_vk != h_ar2);

    // The two auditor channels must yield distinct masks under identical
    // inputs -- if they collide, the same shared-secret `S.x` would leak the
    // same ciphertext on both channels, defeating channel separation.
    assert(h_as != h_ar2);
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
    let mask_bal = poseidon_with_domain(domain::ENCRYPTED_BALANCE, [s, sigma]);
    assert(b_tilde - mask_bal == v);

    let a_tilde = encrypt_allowance(v, s, sigma);
    let mask_allow = poseidon_with_domain(domain::ENCRYPTED_ALLOWANCE, [s, sigma]);
    assert(a_tilde - mask_allow == v);

    let dvk_escrowed = encrypt_esc_dvk(v, s, sigma);
    let mask_esc = poseidon_with_domain(domain::ESCROWED_DELEGATION_VIEWING_KEY, [s, sigma]);
    assert(dvk_escrowed - mask_esc == v);

    let b_tilde_aud_s = encrypt_auditor_sender_balance(v, s, sigma);
    let mask_aud_s = poseidon_with_domain(domain::AUDITOR_SENDER, [s, sigma]);
    assert(b_tilde_aud_s - mask_aud_s == v);
}

#[test]
fn sponge_squeeze_2_first_matches_poseidon_with_domain() {
    // SpongeSqueeze_n shares a single absorb (Section 2.5); the single-squeeze
    // form is just `poseidon_with_domain(d, [s_x, sigma])`, and it must equal
    // the first output of `sponge_squeeze_2` on the same absorb. If this
    // diverges, a circuit doing the single-squeeze form cannot be substituted
    // for the first output of `sponge_squeeze_2` and vice versa.
    let d: Field = domain::AUDITOR_RECIPIENT;
    let s_x: Field = 0x12345;
    let sigma: Field = 0x6789;
    let m2 = sponge_squeeze_2(d, s_x, sigma);
    assert(m2[0] == poseidon_with_domain(d, [s_x, sigma]));
}

#[test]
fn sponge_squeeze_2_outputs_distinct() {
    // The two squeezed masks come from different rate positions of the same
    // permutation. They must differ -- a collision would let an attacker who
    // recovers one mask recover the other, breaking the per-ciphertext
    // independence assumption that justifies the dual-mask construction
    // (Section 8.1, the recipient-auditor ciphertext pair).
    let d: Field = domain::AUDITOR_SENDER;
    let s_x: Field = 0x42;
    let sigma: Field = 0x07;
    let m = sponge_squeeze_2(d, s_x, sigma);
    assert(m[0] != m[1]);
}

#[test]
fn sponge_squeeze_channel_separation() {
    // Same (S.x, sigma) under the two channel domains must produce different
    // mask pairs. This is the property that makes a leaked sender-channel
    // ciphertext useless for decrypting the recipient channel and vice versa.
    let s_x: Field = 0xcafe;
    let sigma: Field = 0xdead;
    let m_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_x, sigma);
    let m_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_x, sigma);
    assert(m_s[0] != m_r[0]);
    assert(m_s[1] != m_r[1]);
}

/// One-shot fixture-generation harness. Prints every primitive's output for a
/// deterministic input set. Captured into `testdata/*.json` (the cross-language
/// contract for the SDK) and mirrored in `fixtures_match_testdata` below. Run
/// `nargo test print_fixtures --show-output` to regenerate.
#[test]
fn print_fixtures() {
    // Inputs (fixed). Any change here invalidates testdata/*.json.
    let sk: Field = 0xdead;
    let addr_f: Field = 0xbeef;
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

    // vk_from_sk(sk, addr_f)
    let vk = vk_from_sk(sk, addr_f);
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

    // encrypt_esc_dvk(dvk, s, op_i)
    let esc = encrypt_esc_dvk(dvk, s_ecdh, op_i);
    println(f"encrypt_esc_dvk = {esc}");

    // encrypt_auditor_sender_balance(v, s, sigma)
    let b_aud_s = encrypt_auditor_sender_balance(v, s_ecdh, sigma);
    println(f"encrypt_auditor_sender_balance = {b_aud_s}");

    // sponge_squeeze_2(AUDITOR_SENDER, s, sigma) -- (amount, balance) masks
    let ss2_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_ecdh, sigma);
    let ss2_s0 = ss2_s[0];
    let ss2_s1 = ss2_s[1];
    println(f"sponge_squeeze_2_AUDITOR_SENDER_0 = {ss2_s0}");
    println(f"sponge_squeeze_2_AUDITOR_SENDER_1 = {ss2_s1}");

    // sponge_squeeze_2(AUDITOR_RECIPIENT, s, sigma) -- (amount, r_tx) masks
    let ss2_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_ecdh, sigma);
    let ss2_r0 = ss2_r[0];
    let ss2_r1 = ss2_r[1];
    println(f"sponge_squeeze_2_AUDITOR_RECIPIENT_0 = {ss2_r0}");
    println(f"sponge_squeeze_2_AUDITOR_RECIPIENT_1 = {ss2_r1}");

    // poseidon_with_domain(VK, [sk, addr_f]) -- direct funnel access, equals vk_from_sk
    let pwd = poseidon_with_domain(domain::VIEWING_KEY, [sk, addr_f]);
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
    let addr_f: Field = 0xbeef;
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

    let vk = vk_from_sk(sk, addr_f);
    assert(vk == 0x208fbdb70d2faacf04f987b54f12aeeaeb432acc29d650c86ce0f6275b958eb8);

    let pvk = pvk_from_vk(vk);
    assert(pvk.x == 0x2e7421c0e86a4c8eed823edf851c364ecee448bf70eae2a95befe3dd9364cc45);
    assert(pvk.y == 0x0b060655740aabef84e7305bb886b75da8d258f346381f03925d69645292fe7a);

    let dvk = dvk_from_vk_op(vk, op_i);
    assert(dvk == 0x1a088264ebf7269160bbf34d5a3f94d7dec37efc609ef76c5dbfa8690af3eae9);

    assert(
        derive_spend_r(vk, sigma)
            == 0x0218f93b7ea0735d4934904f2bf26fc20c33dc01fb032a8f00aa67147132ec91,
    );
    assert(
        derive_allow_r(dvk, sigma_a)
            == 0x0afda20e062a277afc0c86e31ea22597ee2c094b1b9cd08b8d8a2a212a8af1ee,
    );
    assert(
        derive_tx_blind(s_ecdh, sigma)
            == 0x274c4f45d57544efca9f6f505e6b29ae098f1d30243de28f7cef62d05484c83d,
    );

    assert(
        encrypt_amount(v_tx, s_ecdh, sigma)
            == 0x2a07070eb651b03ecc22453ad1cc69a39567ca27fb4224e8727a5fbc83091526,
    );
    assert(
        encrypt_balance(v, vk, sigma)
            == 0x04d1659db899a50a94dcfc54a18b8adaf6e9e8e3046bd11893fbd4a86a7c579c,
    );
    assert(
        encrypt_allowance(v_a, dvk, sigma_a)
            == 0x2440e4fcb575aa4433cd53a37a7396d5022f8c287298cacc0cfc0857250ef02a,
    );
    assert(
        encrypt_esc_dvk(dvk, s_ecdh, op_i)
            == 0x15f3b867cbe94f0b6371e46752431be550f907ba8448c6191b7ba1f47c2cb83e,
    );

    // Per-channel auditor sponge (v0.6, Section 2.5). Values pinned below are
    // produced by `nargo test print_fixtures` against the same fixture inputs.
    let ss2_s = sponge_squeeze_2(domain::AUDITOR_SENDER, s_ecdh, sigma);
    assert(ss2_s[0] == SPONGE_SQUEEZE_2_AUDITOR_SENDER_0);
    assert(ss2_s[1] == SPONGE_SQUEEZE_2_AUDITOR_SENDER_1);
    let ss2_r = sponge_squeeze_2(domain::AUDITOR_RECIPIENT, s_ecdh, sigma);
    assert(ss2_r[0] == SPONGE_SQUEEZE_2_AUDITOR_RECIPIENT_0);
    assert(ss2_r[1] == SPONGE_SQUEEZE_2_AUDITOR_RECIPIENT_1);

    // encrypt_auditor_sender_balance is the canonical composition:
    // v + Poseidon2(AUDITOR_SENDER, [s, sigma]). The mask matches the first
    // squeeze of `sponge_squeeze_2` on the same absorb (single permutation,
    // same state[0]) -- pin against the squeeze_2 vector so the helper and
    // the raw funnel can't drift apart.
    assert(
        encrypt_auditor_sender_balance(v, s_ecdh, sigma)
            == v + SPONGE_SQUEEZE_2_AUDITOR_SENDER_0,
    );
    assert(
        poseidon_with_domain(domain::AUDITOR_SENDER, [s_ecdh, sigma])
            == SPONGE_SQUEEZE_2_AUDITOR_SENDER_0,
    );

    // poseidon_with_domain funnel is the same operation that backs vk_from_sk.
    assert(poseidon_with_domain(domain::VIEWING_KEY, [sk, addr_f]) == vk);
}

// Sponge fixture outputs -- pinned by `print_fixtures` and mirrored in
// `testdata/sponge_squeeze_2.json`. Update both sides together if the lib's
// sponge construction or domain constants change.
global SPONGE_SQUEEZE_2_AUDITOR_SENDER_0: Field =
    0x2787d1e01bbca7828e13e9b2b3fa11cfcfe9c3d2b121b9e17543146822fe1feb;
global SPONGE_SQUEEZE_2_AUDITOR_SENDER_1: Field =
    0x27f3739a132c6353cd5af3edac0ac75faf7fc606acb61367774e4f764ec17b77;
global SPONGE_SQUEEZE_2_AUDITOR_RECIPIENT_0: Field =
    0x0801e6d7184cea11e6225fcfb4e88a90674cb085da8ebc04918c57cc44b9c25d;
global SPONGE_SQUEEZE_2_AUDITOR_RECIPIENT_1: Field =
    0x131187e2ac296c1c54c2be2a5cbe614644f01c754108ce25359212d67b36ecc3;

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
