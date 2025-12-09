use cvlr::nondet::{self, Nondet};
use cvlr_soroban::nondet_address;
use soroban_sdk::{Address, Env, Map, Val, Vec};

use crate::smart_account::Signer;

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
        n = n % (MAX + 1);
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

    // Choose an arbitrary length (but keep it bounded so verification doesn't
    // explode). Adjust MAX as needed.
    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n = n % (MAX + 1);
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

    // Choose an arbitrary length (but keep it bounded so verification doesn't
    // explode). Adjust MAX as needed.
    const MAX: u32 = 5;
    let mut n: u32 = u32::nondet();
    if n > MAX {
        n = n % (MAX + 1);
    }

    let mut out: Map<Address, Val> = Map::new(&env);
    let mut i = 0u32;
    while i < n {
        out.set(nondet_address(), Val::from_payload(u64::nondet()));
        i += 1;
    }

    out
}
