#![cfg(test)]
mod tests {
    use soroban_sdk::{vec, Env, Symbol};

    use diamond_test_utils::{add_facets, setup_diamond_test};

    #[test]
    fn test_diamond_proxy_diamond_cut_add() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        // Upload the diamond proxy WASM
        let proxy_wasm = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/stellar_diamond_proxy.wasm"
        );
        let mut utils = setup_diamond_test!(&env, proxy_wasm);

        let increment_facet = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/shared_storage_1.wasm"
        );

        add_facets!(utils, (increment_facet, ["increment", "decrement", "get_value"]));
    }

    #[test]
    fn test_diamond_proxy_call_functions() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        // Upload the diamond proxy WASM
        let proxy_wasm = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/stellar_diamond_proxy.wasm"
        );
        let mut utils = setup_diamond_test!(&env, proxy_wasm);

        let increment_facet = include_bytes!(
            "../../../../target/wasm32-unknown-unknown/release/shared_storage_1.wasm"
        );

        add_facets!(utils, (increment_facet, ["increment", "decrement", "get_value"]));

        assert_eq!(utils.execute::<u32>("get_value", soroban_sdk::vec![&env]), 0);
        assert_eq!(utils.execute::<u32>("increment", soroban_sdk::vec![&env]), 1);
        assert_eq!(utils.execute::<u32>("decrement", soroban_sdk::vec![&env]), 0);
        assert_eq!(utils.execute::<u32>("get_value", soroban_sdk::vec![&env]), 0);
    }
}
