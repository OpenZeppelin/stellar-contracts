use core::ops::RangeBounds;

use soroban_sdk::{Bytes, BytesN, Env};

/// Extracts and returns a fixed-size array as BytesN<N> from a Bytes object.
///
/// # Arguments
///
/// * `e` - The Soroban environment.
/// * `data` - The Bytes object to extract from.
/// * `r` - The range of bytes to extract.
pub fn extract_from_bytes<const N: usize>(
    e: &Env,
    data: &Bytes,
    r: impl RangeBounds<u32>,
) -> BytesN<N> {
    let buf = data.slice(r).to_buffer::<N>();
    let src = buf.as_slice();
    let mut items = [0u8; N];
    items.copy_from_slice(src);
    BytesN::<N>::from_array(e, &items)
}
