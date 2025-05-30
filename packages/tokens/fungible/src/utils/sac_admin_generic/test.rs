#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, Symbol, Val,
};

use super::{extract_context, get_sac_address, set_sac_address, SacFn};

fn setup_env() -> (Env, Address) {
    let e = Env::default();
    let sac = Address::generate(&e);
    set_sac_address(&e, &sac);
    (e, sac)
}

#[test]
fn test_set_and_get_sac_address() {
    let (e, sac) = setup_env();
    let stored = get_sac_address(&e);
    assert_eq!(stored, sac);
}
