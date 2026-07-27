use stellar_confidential_lib::encrypt_amount;

// Minimal circuit wrapping `encrypt_amount(v_transfer, s, sigma)`. Stand-in for every
// `encrypt_*` helper -- they all share the same shape (Poseidon + add).
fn main(v_transfer: Field, s: pub Field, sigma: pub Field, v_tilde: pub Field) {
    assert(encrypt_amount(v_transfer, s, sigma) == v_tilde);
}
