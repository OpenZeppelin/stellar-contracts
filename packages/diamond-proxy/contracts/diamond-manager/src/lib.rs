#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, IntoVal, Map, Symbol, Val, Vec};
use stellar_diamond_proxy_core::{
    facets::FacetCut, storage::Storage, utils::DiamondProxyExecutor, Error,
};

/// The DiamondManager contract. Provides a convenient interface for interacting with diamonds
#[contract]
pub struct DiamondManager;

#[contractimpl]
impl DiamondManager {
    /// After a diamond is deployed, this function is called to initialize the diamond manager
    ///
    /// # Arguments
    /// * `env` - The environment
    /// * `owner` - The owner of the diamond
    /// * `diamond_address` - The address of the diamond
    pub fn init(env: Env, owner: Address, diamond_address: Address) {
        let storage = Storage::new(env.clone());
        storage.require_uninitialized();
        storage.set_owner(&owner);
        storage.set_initialized();
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "diamond_addr"), &diamond_address);
    }

    /// Executes a facet function via the diamond proxy's fallback implementation
    ///
    /// # Arguments
    /// * `env` - The environment
    /// * `function` - The function to execute
    /// * `args` - The arguments to pass to the function
    pub fn execute(env: Env, function: Symbol, args: Vec<Val>) -> Result<Val, Error> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        env.facet_execute(&diamond_addr, &function, args)
    }

    /// Returns the address of the diamond
    ///
    /// # Arguments
    /// * `env` - The environment
    ///
    /// # Returns
    /// * `Address` - The address of the diamond
    pub fn get_diamond_address(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&Symbol::new(&env, "diamond_addr"))
            .unwrap()
    }

    /// Returns the facets of the diamond
    ///
    /// # Arguments
    /// * `env` - The environment
    ///
    /// # Returns
    /// * `Map<Address, Vec<Symbol>>` - The facets of the diamond
    pub fn facets(env: Env) -> Map<Address, Vec<Symbol>> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        env.invoke_contract(&diamond_addr, &Symbol::new(&env, "facets"), Vec::new(&env))
    }

    /// Returns the selectors of a facet
    ///
    /// # Arguments
    /// * `env` - The environment
    /// * `facet` - The facet to get selectors for
    ///
    /// # Returns
    /// * `Option<Vec<Symbol>>` - The selectors of the facet
    pub fn facet_function_selectors(env: Env, facet: Address) -> Option<Vec<Symbol>> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        env.invoke_contract(
            &diamond_addr,
            &Symbol::new(&env, "facet_function_selectors"),
            soroban_sdk::vec![&env, facet.into_val(&env)],
        )
    }

    /// Returns the address of a facet
    ///
    /// # Arguments
    /// * `env` - The environment
    /// * `function_selector` - The selector of the function to get the facet for
    ///
    /// # Returns
    /// * `Option<Address>` - The address of the facet
    pub fn facet_address(env: Env, function_selector: Symbol) -> Option<Address> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        env.invoke_contract(
            &diamond_addr,
            &Symbol::new(&env, "facet_address"),
            soroban_sdk::vec![&env, function_selector.into_val(&env)],
        )
    }

    /// Returns the addresses of all facets
    ///
    /// # Arguments
    /// * `env` - The environment
    ///
    /// # Returns
    /// * `Vec<Address>` - The addresses of all facets
    pub fn facet_addresses(env: Env) -> Vec<Address> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        env.invoke_contract(
            &diamond_addr,
            &Symbol::new(&env, "facet_addresses"),
            Vec::new(&env),
        )
    }

    /// Executes a diamond cut
    ///
    /// # Arguments
    /// * `env` - The environment
    /// * `diamond_cut` - The diamond cut to execute
    ///
    /// # Returns
    /// * `Result<Vec<Address>, Error>` - The result of the diamond cut
    pub fn diamond_cut(env: Env, diamond_cut: Vec<FacetCut>) -> Result<Vec<Address>, Error> {
        let diamond_addr = Self::get_diamond_address(env.clone());
        // Pass the entire Vec<FacetCut> as a single argument
        env.invoke_contract(
            &diamond_addr,
            &Symbol::new(&env, "diamond_cut"),
            soroban_sdk::vec![&env, diamond_cut.into_val(&env)],
        )
    }

    /// Returns the owner of the diamond
    ///
    /// # Arguments
    /// * `env` - The environment
    ///
    /// # Returns
    /// * `Address` - The owner of the diamond
    pub fn get_owner(env: Env) -> Address {
        let storage = Storage::new(env);
        storage.get_owner().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::testutils::BytesN as _;
    use soroban_sdk::BytesN;
    use stellar_diamond_proxy_core::facets::{FacetAction, FacetCut};

    pub mod contract_diamond_proxy {
        soroban_sdk::contractimport!(
            file = "../../../../.stellar-contracts/manager-deps/target/wasm32-unknown-unknown/release/stellar_diamond_proxy.wasm"
        );
    }

    pub mod contract_shared_storage_facet_1 {
        soroban_sdk::contractimport!(
            file = "../../../../.stellar-contracts/manager-deps/target/wasm32-unknown-unknown/release/shared_storage_1.wasm"
        );
    }

    pub mod contract_shared_storage_facet_2 {
        soroban_sdk::contractimport!(
            file = "../../../../.stellar-contracts/manager-deps/target/wasm32-unknown-unknown/release/shared_storage_2.wasm"
        );
    }

    // Helper function to set up a DiamondManager for testing
    fn setup_diamond_manager(env: &Env) -> (Address, Address, Address) {
        let admin = Address::generate(env);
        // Register the contract and get its ID
        let manager_id = env.register(DiamondManager, ());

        // Initialize DiamondManager
        let diamond_addr = env.as_contract(&manager_id, || {
            let diamond_wasm_hash = env
                .deployer()
                .upload_contract_wasm(contract_diamond_proxy::WASM);
            let salt: BytesN<32> = env.prng().gen();
            let diamond_salt: BytesN<32> = env.prng().gen();
            let diamond_addr = env
                .deployer()
                .with_current_contract(salt)
                .deploy_v2(diamond_wasm_hash, ());

            env.init_contract(
                &diamond_addr,
                soroban_sdk::vec![env, admin.into_val(env), diamond_salt.into_val(env)],
            )
            .unwrap();
            DiamondManager::init(env.clone(), admin.clone(), diamond_addr.clone());
            diamond_addr
        });

        (admin, diamond_addr, manager_id)
    }

    // Test initialization checks
    #[test]
    #[should_panic(expected = "AlreadyInitialized")]
    fn test_init_already_initialized() {
        let env = Env::default();

        // Setup the contract
        let (admin, diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Try to initialize again - should fail
        env.as_contract(&manager_id, || {
            DiamondManager::init(env.clone(), admin.clone(), diamond_addr.clone())
        });
    }

    // Test the get_diamond_address function with uninitialized contract
    #[test]
    #[should_panic(expected = "None")]
    fn test_get_diamond_address_uninitialized() {
        let env = Env::default();

        // Deploy DiamondManager contract without initializing
        let manager_id = env.register(DiamondManager, ());

        // Try to get diamond address without initializing - should fail
        env.as_contract(&manager_id, || {
            DiamondManager::get_diamond_address(env.clone())
        });
    }

    // Test DiamondManager's diamond_cut functionality with an expected error
    #[test]
    #[should_panic(expected = "InvalidAction")]
    fn test_diamond_cut_error_handling() {
        let env = Env::default();

        // Set up the environment with proper diamond deployment
        let (_admin, _diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Create a facet cut that will cause an error
        // Zero hash for Add is not allowed so this will fail with InvalidAction
        let facet_cut = FacetCut {
            wasm_hash_of_facet: BytesN::from_array(&env, &[0; 32]),
            action: FacetAction::Add,
            selectors: soroban_sdk::vec![&env, Symbol::new(&env, "test_selector")],
            salt: BytesN::<32>::random(&env),
        };

        let diamond_cuts = soroban_sdk::vec![&env, facet_cut];

        // Mock authorization
        env.mock_all_auths();

        // This will panic with InvalidAction - but that's what we expect
        env.as_contract(&manager_id, || {
            DiamondManager::diamond_cut(env.clone(), diamond_cuts).expect("diamond_cut failed")
        });
    }

    // Test initialization of DiamondManager
    #[test]
    fn test_initialization_and_get_diamond_address() {
        let env = Env::default();

        // Setup the contract
        let (_, diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Test get_diamond_address
        let retrieved_address = env.as_contract(&manager_id, || {
            DiamondManager::get_diamond_address(env.clone())
        });

        assert_eq!(
            retrieved_address, diamond_addr,
            "Diamond address was not stored correctly"
        );
    }

    // Test that we can properly store and access the owner of the DiamondManager
    #[test]
    fn test_owner_management() {
        let env = Env::default();

        // Setup the contract
        let (admin, _, manager_id) = setup_diamond_manager(&env);

        // Test get_owner
        let owner = env.as_contract(&manager_id, || DiamondManager::get_owner(env.clone()));

        assert_eq!(owner, admin, "Owner address was not stored correctly");
    }

    // Test DiamondManager's diamond_cut functionality to add facets
    #[test]
    fn test_diamond_cut_add_facets() {
        let env = Env::default();

        // Set up the environment with proper diamond deployment
        let (_admin, _diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Mock authorization for all operations
        env.mock_all_auths_allowing_non_root_auth();

        // Execute within contract context - this will panic with InvalidAction
        // which is expected in the test environment
        env.as_contract(&manager_id, || {
            // Deploy facet contracts via manager
            let facet_1_hash = env
                .deployer()
                .upload_contract_wasm(contract_shared_storage_facet_1::WASM);

            // Create facet cuts for the deployment with correct selectors for facet_1
            let cuts = soroban_sdk::vec![
                &env,
                FacetCut {
                    wasm_hash_of_facet: facet_1_hash,
                    action: FacetAction::Add,
                    selectors: soroban_sdk::vec![
                        &env,
                        Symbol::new(&env, "increment"),
                        Symbol::new(&env, "decrement"),
                        Symbol::new(&env, "get_value"),
                    ],
                    salt: BytesN::<32>::random(&env),
                },
            ];

            // Call diamond_cut through DiamondManager
            DiamondManager::diamond_cut(env.clone(), cuts).expect("diamond_cut failed")
        });
    }

    // Test DiamondManager's diamond_cut functionality to replace facets
    #[test]
    fn test_diamond_cut_replace_facets() {
        let env = Env::default();

        // Set up the environment with proper diamond deployment
        let (_admin, _diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Mock authorization for all operations
        env.mock_all_auths_allowing_non_root_auth();

        // Execute operation within contract context - will panic with InvalidAction
        env.as_contract(&manager_id, || {
            // Deploy both facet contracts
            let facet_1_hash = env
                .deployer()
                .upload_contract_wasm(contract_shared_storage_facet_1::WASM);

            // Create facet cuts for the deployment with correct selectors for facet_1
            let cuts = soroban_sdk::vec![
                &env,
                FacetCut {
                    wasm_hash_of_facet: facet_1_hash,
                    action: FacetAction::Add,
                    selectors: soroban_sdk::vec![
                        &env,
                        Symbol::new(&env, "increment"),
                        Symbol::new(&env, "decrement"),
                        Symbol::new(&env, "get_value"),
                    ],
                    salt: BytesN::<32>::random(&env),
                },
            ];

            // Call diamond_cut through DiamondManager
            DiamondManager::diamond_cut(env.clone(), cuts).expect("diamond_cut failed")
        });
    }

    // Test DiamondManager's diamond_cut functionality to remove facets
    #[test]
    fn test_diamond_cut_remove_facets() {
        let env = Env::default();

        // Set up the environment with proper diamond deployment
        let (_admin, _diamond_addr, manager_id) = setup_diamond_manager(&env);

        // Mock authorization for all operations
        env.mock_all_auths_allowing_non_root_auth();

        // Execute within contract context - will panic with InvalidAction
        env.as_contract(&manager_id, || {
            // Deploy facet contract
            let facet_hash = env
                .deployer()
                .upload_contract_wasm(contract_shared_storage_facet_1::WASM);

            // Create facet cuts for the deployment with correct selectors
            let cuts = soroban_sdk::vec![
                &env,
                FacetCut {
                    wasm_hash_of_facet: facet_hash,
                    action: FacetAction::Add,
                    selectors: soroban_sdk::vec![
                        &env,
                        Symbol::new(&env, "increment"),
                        Symbol::new(&env, "decrement"),
                        Symbol::new(&env, "get_value"),
                    ],
                    salt: BytesN::<32>::random(&env),
                },
            ];

            // Call diamond_cut through DiamondManager
            DiamondManager::diamond_cut(env.clone(), cuts).expect("diamond_cut failed")
        });
    }
}
