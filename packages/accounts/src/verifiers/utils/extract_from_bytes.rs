use core::ops::{Bound, RangeBounds};

use soroban_sdk::{Bytes, BytesN, Env};

/// Extracts and returns a fixed-size array as `Option<BytesN<N>>` from a
/// `Bytes` object or `None` if range is out of bounds or N is too small to fit
/// the extracted slice.
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
) -> Option<BytesN<N>> {
    let start = match r.start_bound().cloned() {
        Bound::Unbounded => 0,
        Bound::Included(n) | Bound::Excluded(n) => n,
    };
    let end = match r.end_bound().cloned() {
        Bound::Unbounded => data.len(),
        Bound::Included(n) => n + 1,
        Bound::Excluded(n) => n,
    };
    if end > data.len() || end - start != N as u32 {
        return None;
    }

    let buf = data.slice(r).to_buffer::<N>();
    let mut items = [0u8; N];
    items.copy_from_slice(buf.as_slice());

    Some(BytesN::<N>::from_array(e, &items))
}
