use crate::{
    assert_on_curve, assert_on_curve_non_identity, assert_range_128, commit, derive_allow_r,
    derive_spend_r, derive_tx_blind, domain, dvk_from_vk_op, ecdh, encrypt_allowance,
    encrypt_amount, encrypt_auditor_bal, encrypt_auditor_tx, encrypt_balance, encrypt_esc_dvk,
    G, H, poseidon_with_domain, pvk_from_vk, scalar_mul, vk_from_sk,
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
    assert_on_curve(G);
    assert_on_curve(H);
    assert_on_curve_non_identity(G);
    assert_on_curve_non_identity(H);
}

#[test]
fn on_curve_accepts_identity_lenient_only() {
    let o = EmbeddedCurvePoint { x: 0, y: 0, is_infinite: true };
    // Identity is allowed for commitments (e.g. fresh accounts with v=r=0).
    assert_on_curve(o);
}

#[test(should_fail)]
fn on_curve_rejects_off_curve() {
    // (1, 2) is not on Grumpkin: 2^2 = 4, 1^3 - 17 = -16. 4 != -16.
    let bad = EmbeddedCurvePoint { x: 1, y: 2, is_infinite: false };
    assert_on_curve(bad);
}

#[test(should_fail)]
fn on_curve_rejects_non_canonical_identity() {
    // Identity must be encoded as (0, 0, is_infinite=true). A prover declaring
    // (42, 7, is_infinite=true) would leave x, y unconstrained without the
    // canonical-encoding asserts and could later trick field-wise coordinate
    // comparisons in callers.
    let bad = EmbeddedCurvePoint { x: 42, y: 7, is_infinite: true };
    assert_on_curve(bad);
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
