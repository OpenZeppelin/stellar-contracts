use stellar_confidential_lib::sponge_squeeze_2;

// Minimal circuit wrapping `sponge_squeeze_2(d, s_x, sigma) -> [Field; 2]`.
// Used by `nargo info` to report the per-call constraint cost of the
// two-squeeze auditor channel sponge (Section 2.5). The one-squeeze case
// shares the `poseidon_with_domain` gadget already in this directory.
fn main(d: pub Field, s_x: pub Field, sigma: pub Field, m0: pub Field, m1: pub Field) {
    let out = sponge_squeeze_2(d, s_x, sigma);
    assert(out[0] == m0);
    assert(out[1] == m1);
}
