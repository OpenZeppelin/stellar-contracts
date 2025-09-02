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
    if end > data.len() || end - start > N as u32 {
        return None;
    }

    let buf = data.slice(r).to_buffer::<N>();
    let mut items = [0u8; N];
    items.copy_from_slice(buf.as_slice());

    Some(BytesN::<N>::from_array(e, &items))
}

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn base64_url_encode(dst: &mut [u8], src: &[u8]) {
    let mut di: usize = 0;
    let mut si: usize = 0;
    let n = (src.len() / 3) * 3; // (.. / 3 * 3) to ensure a % 3 `n`

    while si < n {
        let val = (src[si] as usize) << 16 | (src[si + 1] as usize) << 8 | (src[si + 2] as usize);
        dst[di] = ALPHABET[val >> 18 & 0x3F];
        dst[di + 1] = ALPHABET[val >> 12 & 0x3F];
        dst[di + 2] = ALPHABET[val >> 6 & 0x3F];
        dst[di + 3] = ALPHABET[val & 0x3F];
        si += 3;
        di += 4;
    }

    let remain = src.len() - si;

    if remain == 0 {
        return;
    }

    let mut val = (src[si] as usize) << 16;

    if remain == 2 {
        val |= (src[si + 1] as usize) << 8;
    }

    dst[di] = ALPHABET[val >> 18 & 0x3F];
    dst[di + 1] = ALPHABET[val >> 12 & 0x3F];

    if remain == 2 {
        dst[di + 2] = ALPHABET[val >> 6 & 0x3F];
    }
}
