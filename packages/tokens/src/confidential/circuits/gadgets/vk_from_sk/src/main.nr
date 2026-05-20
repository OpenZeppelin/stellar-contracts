use stellar_confidential_lib::vk_from_sk;

// Minimal circuit wrapping `vk_from_sk(sk, wrap) = Poseidon2(VK, sk, wrap)`.
fn main(sk: Field, wrap: pub Field, vk: pub Field) {
    assert(vk_from_sk(sk, wrap) == vk);
}

#[test]
fn matches_fixture() {
    // Pinned from testdata/vk_from_sk.json: vk_from_sk(0xdead, 0xbeef).
    main(
        0xdead,
        0xbeef,
        0x0ac6ade91aea36f4de35eb5a06a67e1b9c3dab9842479f1cce85caf826194ec2,
    );
}
