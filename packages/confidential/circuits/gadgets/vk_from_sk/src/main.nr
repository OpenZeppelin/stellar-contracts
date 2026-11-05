use stellar_confidential_lib::vk_from_sk;

// Minimal circuit wrapping `vk_from_sk(sk, addr_f) = Poseidon2(VK, sk, addr_f)`.
fn main(sk: Field, addr_f: pub Field, vk: pub Field) {
    assert(vk_from_sk(sk, addr_f) == vk);
}
