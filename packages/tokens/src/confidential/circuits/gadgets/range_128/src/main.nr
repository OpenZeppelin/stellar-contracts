use stellar_confidential_lib::assert_range_128;

// Minimal circuit wrapping `assert_range_128(x)`.
fn main(x: Field) {
    assert_range_128(x);
}

#[test]
fn accepts_max() {
    main(0xffffffffffffffffffffffffffffffff); // 2^128 - 1
}

#[test]
fn accepts_zero() {
    main(0);
}

#[test(should_fail)]
fn rejects_2_pow_128() {
    main(0x100000000000000000000000000000000);
}
