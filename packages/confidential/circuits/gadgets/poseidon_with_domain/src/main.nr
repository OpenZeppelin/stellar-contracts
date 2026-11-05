use stellar_confidential_lib::poseidon_with_domain;

// Minimal circuit wrapping the Poseidon2 funnel with 2 absorbed inputs (the
// arity used by `vk_from_sk`, `dvk_from_vk_op`, all `derive_*` and most
// `encrypt_*` helpers).
fn main(d: pub Field, a: Field, b: Field, out: pub Field) {
    assert(poseidon_with_domain(d, [a, b]) == out);
}
