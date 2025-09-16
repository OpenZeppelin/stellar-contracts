extern crate std;

use soroban_sdk::contractimport;

// use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use stellar_tokens::rwa::identity_registry_storage::{CountryCode, CountryData};

// mod claim_issuer {
//     soroban_sdk::contractimport!(file = "./testdata/claim_issuer_example.wasm");
// }

mod contract_v1 {
    soroban_sdk::contractimport!(file = "./testdata/upgradeable_v1_example.wasm");
}

#[test]
fn test_integration() {}
