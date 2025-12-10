use cvlr::nondet::{self, Nondet};
use cvlr_soroban::{
    nondet_address, nondet_bytes, nondet_bytes_n, nondet_string, nondet_symbol, nondet_vec,
};
use soroban_sdk::{
    auth::{
        Context, ContractContext, ContractExecutable, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    crypto::Hash,
    Address, Bytes, BytesN, Env, IntoVal, Map, Symbol, Val, Vec,
};

use crate::smart_account::{Signatures, Signer};

pub fn nondet_signers_vec() -> Vec<Signer>
where
    Signer: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
        + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    let env = Env::default();

    // Choose an arbitrary length (but keep it bounded so verification doesn't
    // explode). Adjust MAX as needed.
    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<Signer> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        out.push_back(Signer::nondet());
        i += 1;
    }

    out
}

pub fn nondet_policy_vec() -> Vec<Address> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<Address> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        out.push_back(nondet_address());
        i += 1;
    }

    out
}

pub fn nondet_policy_map() -> Map<Address, Val> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Map<Address, Val> = Map::new(&env);
    let mut i = 0u32;
    while i < n {
        out.set(nondet_address(), Val::from_payload(u64::nondet()));
        i += 1;
    }

    out
}

pub fn nondet_hash_32() -> Hash<32> {
    let e = Env::default();
    let bytes = nondet_bytes();
    e.crypto().sha256(&bytes)
}

pub fn nondet_signatures_map() -> Signatures {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Map<Signer, Bytes> = Map::new(&env);
    let mut i = 0u32;
    while i < n {
        out.set(Signer::nondet(), nondet_bytes());
        i += 1;
    }

    Signatures(out)
}

pub fn nondet_context() -> Context {
    let e = Env::default();
    match u8::nondet() % 3 {
        0 => Context::Contract(ContractContext {
            contract: nondet_address(),
            fn_name: nondet_symbol(), // maybe should use symbol_short!("foo")?
            args: ().into_val(&e),
        }),
        1 => Context::CreateContractHostFn(CreateContractHostFnContext {
            salt: nondet_bytes_n(),
            executable: ContractExecutable::Wasm(nondet_bytes_n()),
        }),
        _ => Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
            salt: nondet_bytes_n(),
            executable: ContractExecutable::Wasm(nondet_bytes_n()),
            constructor_args: ().into_val(&e),
        }),
    }
}

pub fn nondet_context_vec() -> Vec<Context> {
    let env = Env::default();

    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n %= MAX + 1;
    }

    let mut out: Vec<Context> = Vec::new(&env);
    let mut i = 0u32;
    while i < n {
        out.push_back(nondet_context());
        i += 1;
    }

    out
}
