use cvlr::{clog, log::CvlrLog};
use soroban_sdk::{Address, BytesN, IntoVal, TryFromVal, Val, Vec};
use crate::rwa::claim_issuer::storage::SigningKey;

pub fn clog_vec<T>(vec: &Vec<T>) 
where
    T: CvlrLog + IntoVal<soroban_sdk::Env, Val> + TryFromVal<soroban_sdk::Env, Val>,
{
    let length = vec.len();
    clog!(length);
    let mut i = 0;
    while i < length {
        let element = vec.get(i);
        if let Some(element) = element {
            clog!(element);
        }
        i = i + 1;
    }
}

pub fn clog_vec_addresses(vec_addresses: &Vec<Address>) {
    // important to put the clogs in optional because i don't want to prevent the case of empty vector by clogs
    let length = vec_addresses.len();
    clog!(length);
    let mut i = 0;
    while i < length {
        let address = vec_addresses.get(i);
        if let Some(address) = address {
            clog!(cvlr_soroban::Addr(&address));
        }
        i = i + 1;
    }
}

pub fn clog_vec_bytes_n(vec_bytes_n: &Vec<BytesN<32>>) {
    let length = vec_bytes_n.len();
    clog!(length);
    let mut i = 0;
    while i < length {
        let bytes_n = vec_bytes_n.get(i);
        if let Some(bytes_n) = bytes_n {
            clog!(cvlr_soroban::BN(&bytes_n));
        }
        i = i + 1;
    }
}

pub fn clog_vec_signing_keys(vec_signing_keys: &Vec<SigningKey>) {
    let length = vec_signing_keys.len();
    clog!(length);
    let mut i = 0;
    while i < length {
        let signing_key = vec_signing_keys.get(i);
        if let Some(signing_key) = signing_key {
            clog!(cvlr_soroban::B(&signing_key.public_key));
            clog!(signing_key.scheme);
        }
        i = i + 1;
    }
}