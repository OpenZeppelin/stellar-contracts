#![cfg(test)]
mod tests {
    use diamond_test_utils::setup_diamond_test;
    use soroban_sdk::{vec, Env, Symbol};

    #[test]
    fn test_authorization_debug() {
        let env = Env::default();
        env.mock_all_auths();

        // Setup diamond proxy
        let proxy_wasm = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/stellar_diamond_proxy.wasm"
        );
        let mut utils = setup_diamond_test!(&env, proxy_wasm);

        // Add facet with increment function
        let increment_facet = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/shared_storage_1.wasm"
        );
        let facet_addr = utils.add_facet(
            increment_facet,
            vec![&env, Symbol::new(&env, "increment"), Symbol::new(&env, "get_value")],
        );

        println!("Diamond address: {:?}", utils.diamond_id);
        println!("Facet address: {facet_addr:?}");

        // Call through proxy - should work
        let result_via_proxy: u32 = utils.execute("increment", vec![&env]);
        println!("Result via proxy: {result_via_proxy}");

        // Try to call require_diamond_proxy_caller directly
        let security_check_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result: Result<(), stellar_diamond_proxy_core::Error> = env.invoke_contract(
                &facet_addr,
                &Symbol::new(&env, "require_diamond_proxy_caller"),
                vec![&env],
            );
            result
        }));

        println!("Direct security check result: {security_check_result:?}");

        // Print all authorizations
        let auths = env.auths();
        println!("Number of authorizations: {}", auths.len());
        for (i, auth) in auths.iter().enumerate() {
            println!("Auth {i}: {auth:?}");
        }
    }
}
