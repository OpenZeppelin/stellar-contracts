use stellar_confidential_lib::vk_from_sk;

// Minimal circuit wrapping `vk_from_sk(sk, wrap) = Poseidon2(VK, sk, wrap)`.
fn main(sk: Field, wrap: pub Field, vk: pub Field) {
    assert(vk_from_sk(sk, wrap) == vk);
}
