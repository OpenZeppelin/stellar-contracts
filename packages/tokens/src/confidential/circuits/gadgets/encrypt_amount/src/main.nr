use stellar_confidential_lib::encrypt_amount;

// Minimal circuit wrapping `encrypt_amount(v_tx, s, sigma)`. Stand-in for every
// `encrypt_*` helper -- they all share the same shape (Poseidon + add).
fn main(v_tx: Field, s: pub Field, sigma: pub Field, v_tilde: pub Field) {
    assert(encrypt_amount(v_tx, s, sigma) == v_tilde);
}

#[test]
fn matches_fixture() {
    // Pinned from testdata/encrypt_amount.json: encrypt_amount(100, 0x12345, 1).
    main(
        100,
        0x12345,
        0x01,
        0x274c4f45d57544efca9f6f505e6b29ae098f1d30243de28f7cef62d05484c8a1,
    );
}
