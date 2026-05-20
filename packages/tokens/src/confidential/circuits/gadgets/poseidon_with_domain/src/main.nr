use stellar_confidential_lib::poseidon_with_domain;

// Minimal circuit wrapping the Poseidon2 funnel with 2 absorbed inputs (the
// arity used by `vk_from_sk`, `dvk_from_vk_op`, all `derive_*` and most
// `encrypt_*` helpers).
fn main(d: pub Field, a: Field, b: Field, out: pub Field) {
    assert(poseidon_with_domain(d, [a, b]) == out);
}

#[test]
fn matches_fixture() {
    // Pinned from testdata/poseidon_with_domain.json -- same value as
    // vk_from_sk(0xdead, 0xbeef) since domain::VK = 1 and the inputs match.
    main(
        0x01,
        0xdead,
        0xbeef,
        0x0ac6ade91aea36f4de35eb5a06a67e1b9c3dab9842479f1cce85caf826194ec2,
    );
}
