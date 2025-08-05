#![cfg(test)]
use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, Symbol};
use stellar_diamond_proxy_core::{storage::Storage, Error};
use stellar_facet_macro::facet;

#[contract]
pub struct TestFacet;

#[facet]
#[contractimpl]
impl TestFacet {
    // The init function will be automatically generated here
    pub fn test_function(_env: Env, value: Symbol) -> Result<Symbol, Error> {
        // Just a simple test function
        Ok(value)
    }
}

#[test]
fn test_facet_macro() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let dummy_diamond_address = Address::generate(&env);
    let contract_id = env.register(TestFacet, ());
    // TODO: contract_import! real bytes for shared storage addr
    let shared_storage_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize contract using the auto-generated init function
        TestFacet::init(
            env.clone(),
            owner.clone(),
            shared_storage_address,
            dummy_diamond_address.clone(),
        )
        .unwrap();

        // Verify storage is initialized correctly
        let storage = Storage::new(env.clone());
        let stored_owner = storage.get_owner().unwrap();
        assert_eq!(stored_owner, owner);
    });

    // Test that direct calls to facet functions fail due to security checks
    // This is the expected behavior - facets should only be callable through the diamond proxy
    let test_value = Symbol::new(&env, "test_value");

    // We expect this to panic with an authorization error
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        env.invoke_contract::<Result<Symbol, Error>>(
            &contract_id,
            &Symbol::new(&env, "test_function"),
            soroban_sdk::vec![&env, test_value.to_val()],
        )
    }));

    // The call should have panicked due to authorization failure
    assert!(result.is_err());
}
