use soroban_sdk::{xdr::ToXdr, Bytes, Env, Symbol, TryIntoVal, Val};

pub trait SorobanConcat {
    fn concat(self, env: &Env, other: impl TryIntoVal<Env, Val>) -> Symbol;
}

const MAX_BUFFER_SIZE: usize = 256;
const VALS: [u8; 63] = [
    b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p',
    b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z', b'A', b'B', b'C', b'D', b'E', b'F',
    b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V',
    b'W', b'X', b'Y', b'Z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'_',
];

impl<T: TryIntoVal<Env, Val>> SorobanConcat for T {
    fn concat(self, env: &Env, other: impl TryIntoVal<Env, Val>) -> Symbol {
        let this = self
            .try_into_val(env)
            .expect("Failed to convert `this` to Val")
            .to_xdr(env)
            .to_buffer::<{ MAX_BUFFER_SIZE }>();

        // Append other to self
        let other: Val = other
            .try_into_val(env)
            .expect("Failed to convert `other` to ScVal");
        let other = other.to_xdr(env).to_buffer::<{ MAX_BUFFER_SIZE }>();

        let mut slice = Bytes::new(env);
        let this_slice = this.as_slice();
        let other_slice = other.as_slice();

        slice.extend_from_slice(this_slice);
        slice.extend_from_slice(other_slice);

        let mut hash: [u8; 32] = env.crypto().keccak256(&slice).to_bytes().to_array();

        // Convert every byte in 'hash' to be either a-z, A-Z, 0-9, or _. There are 32 slots, and each slot can hold 63 possible values,
        // therefore there are 63^32 possible unique values = 3.8 * 10^57
        for byte in &mut hash {
            *byte = VALS[*byte as usize % VALS.len()];
        }

        // SAFETY:
        // * We have already ensured every value falls within a-z, A-Z, 0-9, or _. which are all subsets of UTF-8,
        // therefore the unchecked.
        // * While the lifetime of the pointee, 'hash', lasts as long as this function, Symbol::new
        // will clone the values and store internally without taking valued by reference.
        let symbol_str = unsafe { core::str::from_utf8_unchecked(&hash) };
        Symbol::new(env, symbol_str)
    }
}
