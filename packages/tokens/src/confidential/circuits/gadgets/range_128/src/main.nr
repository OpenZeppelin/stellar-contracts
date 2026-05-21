use stellar_confidential_lib::assert_range_128;

// Minimal circuit wrapping `assert_range_128(x)`.
fn main(x: Field) {
    assert_range_128(x);
}
