extern crate std;

use soroban_sdk::contractimport;

// use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
// use stellar_tokens::rwa::identity_registry_storage::CountryData;

mod claim_issuer {
    soroban_sdk::contractimport!(file = "../testdata/rwa_token_example.wasm");
}

#[test]
fn test_integration() {}
