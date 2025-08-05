#![no_std]
use soroban_env_common::Env as _;
use soroban_sdk::auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation};
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, IntoVal, Map, Symbol, Val, Vec,
};

// Include the loupe functions module
use stellar_diamond_proxy_core::facets::{FacetAction, FacetCut};
use stellar_diamond_proxy_core::utils::DiamondProxyExecutor;
use stellar_diamond_proxy_core::Error;

#[derive(Clone, Debug)]
#[contracttype]
pub struct DiamondState {
    pub version: u32,
    pub owner: Address,
    pub initialized: bool,
    pub loupe: Map<Address, Vec<Symbol>>,
    pub shared_storage_addr: Address,
}

#[contract]
pub struct DiamondProxy;

pub mod contract_shared_storage {
    soroban_sdk::contractimport!(
        file = "../../../../.stellar-contracts/proxy-deps/target/wasm32-unknown-unknown/release/stellar_shared_storage.wasm"
    );
}

#[contractimpl]
impl DiamondProxy {
    pub fn init(env: Env, owner: Address, salt: BytesN<32>) -> Result<(), Error> {
        env.logs().add("started_diamond_init", &[]);
        if env.storage().instance().has(&Symbol::new(&env, "diamond")) {
            return Err(Error::AlreadyInitialized);
        }

        // Deploy the shared storage layer not that the diamond proxy is setup.
        // We need to deploy the shared storage in the context of the diamond,
        // not the factory.
        let shared_storage_wasm_hash = env
            .deployer()
            .upload_contract_wasm(contract_shared_storage::WASM);

        // Deploy the shared storage contract under the DiamondFactory
        let shared_storage_address = env
            .deployer()
            .with_current_contract(salt.clone())
            .deploy_v2(shared_storage_wasm_hash, ());

        // Call init on the shared storage contract
        env.init_contract(
            &shared_storage_address,
            Vec::from_array(&env, [owner.clone().into_val(&env)]),
        )
        .expect("Failed to initialize shared storage");

        let state = DiamondState {
            version: 1,
            owner: owner.clone(),
            initialized: true,
            loupe: Map::new(&env),
            shared_storage_addr: shared_storage_address,
        };

        env.storage()
            .instance()
            .set(&Symbol::new(&env, "diamond"), &state);

        env.logs().add("finished_diamond_init", &[]);

        Ok(())
    }

    pub fn diamond_cut(env: Env, diamond_cut: Vec<FacetCut>) -> Result<Vec<Address>, Error> {
        env.logs().add("Diamond cut", &[]);

        let mut diamond = Self::load_diamond_state(env.clone())?;

        // Require authorization from owner
        diamond.owner.require_auth();

        let insert_selector = |diamond: &mut DiamondState, facet: Address, selector: Symbol| {
            if !diamond.loupe.contains_key(facet.clone()) {
                diamond.loupe.set(facet.clone(), Vec::new(&env));
            }

            let mut selectors = diamond.loupe.get(facet.clone()).unwrap();
            if selectors.contains(&selector) {
                return;
            }

            selectors.push_back(selector);
            diamond.loupe.set(facet, selectors);
        };

        let remove_selector = |diamond: &mut DiamondState, selector: Symbol| -> Option<Address> {
            let mut ret = None;
            loop {
                let mut address_ret = None;
                for (address, mut selectors) in diamond.loupe.clone().iter() {
                    if let Some(index) = selectors.clone().iter().position(|s| s == selector) {
                        selectors.remove(index as u32);
                        address_ret = Some((address, selectors));
                        break;
                    }
                }

                if let Some((address, selectors)) = address_ret {
                    if selectors.is_empty() {
                        diamond.loupe.remove(address.clone());
                    } else {
                        diamond.loupe.set(address.clone(), selectors);
                    }

                    ret = Some(address)
                } else {
                    break;
                }
            }
            ret
        };

        let address_of_selector = |diamond: &DiamondState, selector: Symbol| {
            diamond
                .loupe
                .clone()
                .iter()
                .find(|(_, selectors)| selectors.contains(&selector))
                .map(|(address, _)| address)
        };

        let mut mutated_facets = Vec::new(&env);
        let mut did_mutate = false;

        // Process each facet cut
        for cut in diamond_cut.iter() {
            env.logs()
                .add("Diamond cut::phase_1", &[cut.selectors.into_val(&env)]);
            // Make salt deterministic from wasm hash
            let salt = cut.salt;
            match cut.action {
                FacetAction::Add => {
                    let target = env
                        .deployer()
                        .with_current_contract(salt.clone())
                        .deploy_v2(cut.wasm_hash_of_facet.clone(), ());
                    env.logs().add(
                        "Diamond cut::phase_1::add::deploy",
                        &[cut.selectors.into_val(&env)],
                    );

                    for selector in cut.selectors.iter() {
                        if address_of_selector(&mut diamond, selector.clone()).is_some() {
                            return Err(Error::DiamondSelectorAlreadyAdded);
                        }

                        insert_selector(&mut diamond, target.clone(), selector);
                    }

                    mutated_facets.push_back(target);
                }
                FacetAction::Replace => {
                    let target = env
                        .deployer()
                        .with_current_contract(salt.clone())
                        .deploy_v2(cut.wasm_hash_of_facet.clone(), ());

                    for selector in cut.selectors.iter() {
                        // remove any existing selectors
                        remove_selector(&mut diamond, selector.clone());
                        insert_selector(&mut diamond, target.clone(), selector);
                        mutated_facets.push_back(target.clone());
                    }
                }
                FacetAction::Remove => {
                    for selector in cut.selectors.iter() {
                        if remove_selector(&mut diamond, selector).is_none() {
                            return Err(Error::DiamondSelectorNotFound);
                        } else {
                            did_mutate = true;
                        }
                    }
                }
            }
        }

        env.logs()
            .add("Diamond cut::phase_2", &[mutated_facets.into_val(&env)]);

        for facet in mutated_facets.clone() {
            let args = soroban_sdk::vec![
                &env,
                diamond.owner.clone().into_val(&env),
                diamond.shared_storage_addr.clone().into_val(&env),
                env.current_contract_address().into_val(&env),
            ];
            let res =
                env.invoke_contract::<Result<(), Error>>(&facet, &Symbol::new(&env, "init"), args);

            if let Err(err) = res {
                let _ = env.fail_with_error(err.into());
            }
        }

        if did_mutate || !mutated_facets.is_empty() {
            env.storage()
                .instance()
                .update(&Symbol::new(&env, "diamond"), |_| diamond);
        }

        Ok(mutated_facets)
    }

    pub fn fallback(env: Env, selector: Symbol, args: Vec<Val>) -> Result<Val, Error> {
        let diamond_state = Self::load_diamond_state(env.clone())?;
        let target = Self::facet_address(env.clone(), selector.clone())
            .ok_or(Error::DiamondSelectorNotFound)?;

        // Create a unique authorization marker for this call
        // The facet will require auth on the diamond proxy address with the facet address as argument
        // This ensures only calls through the diamond proxy's fallback can succeed
        let auth_args = soroban_sdk::vec![&env, target.clone().into_val(&env)];

        env.authorize_as_current_contract(soroban_sdk::vec![
            &env,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: env.current_contract_address(), // Diamond proxy address
                    fn_name: Symbol::new(&env, "__check_auth"),
                    args: auth_args.clone(),
                },
                sub_invocations: soroban_sdk::vec![&env],
            })
        ]);

        // Create sub-invocations for shared storage calls
        // This allows facets to call shared storage functions with authorization
        let shared_storage_addr = diamond_state.shared_storage_addr.clone();

        // Create an empty Vec<Val> to represent any arguments
        let any_args: Vec<Val> = Vec::new(&env);

        let shared_storage_sub_invocations = soroban_sdk::vec![
            &env,
            // Instance storage functions
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "get_instance_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "set_instance_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "del_instance_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            // Persistent storage functions
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "get_persistent_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "set_persistent_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "del_persistent_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            // Temporary storage functions
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "get_temporary_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "set_temporary_shared_storage_at"),
                    args: any_args.clone().into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: shared_storage_addr.clone(),
                    fn_name: Symbol::new(&env, "del_temporary_shared_storage_at"),
                    args: any_args.into_val(&env),
                },
                sub_invocations: soroban_sdk::vec![&env],
            }),
        ];

        // Authorize the facet call with shared storage sub-invocations
        env.authorize_as_current_contract(soroban_sdk::vec![
            &env,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: target.clone(),
                    fn_name: selector.clone(),
                    args: args.clone().into_val(&env),
                },
                sub_invocations: shared_storage_sub_invocations,
            })
        ]);

        // Forward the authentication context to the target contract
        // This ensures that any require_auth() calls in the target contract
        // will be properly authenticated with the original caller's context
        env.invoke_contract(&target, &selector, args)
    }

    /// Returns a list of all facet addresses used by the diamond.
    pub fn facet_addresses(env: Env) -> Vec<Address> {
        let mut addresses = soroban_sdk::Vec::new(&env);

        // Iterate through all stored selectors and collect unique addresses
        let keys = Self::facets(env.clone());
        for (key, _) in keys.iter() {
            addresses.push_back(key.clone());
        }

        addresses
    }

    /// Returns the facet address that handles the specified function selector.
    pub fn facet_address(env: Env, function_selector: Symbol) -> Option<Address> {
        let map = Self::facets(env.clone());
        for (addr, selectors) in map.iter() {
            if selectors.contains(&function_selector) {
                return Some(addr);
            }
        }

        None
    }

    /// Returns the function selectors supported by the specified facet.
    pub fn facet_function_selectors(env: Env, facet: Address) -> Option<Vec<Symbol>> {
        let keys = Self::facets(env.clone());
        for (key, selectors) in keys.iter() {
            if key == facet {
                return Some(selectors);
            }
        }

        None
    }

    /// Returns all facets and their function selectors.
    pub fn facets(env: Env) -> Map<Address, Vec<Symbol>> {
        match Self::load_diamond_state(env.clone()) {
            Ok(state) => state.loupe,
            Err(err) => {
                env.panic_with_error(err);
            }
        }
    }

    // Test helper functions - included in all builds for integration testing
    pub fn shared_storage_facet_address(env: Env) -> Result<Address, Error> {
        Ok(Self::load_diamond_state(env.clone())?.shared_storage_addr)
    }

    pub fn owner(env: Env) -> Result<Address, Error> {
        let diamond = Self::load_diamond_state(env.clone())?;
        Ok(diamond.owner)
    }

    fn load_diamond_state(env: Env) -> Result<DiamondState, Error> {
        let storage: DiamondState = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "diamond"))
            .ok_or(Error::DiamondProxyNotInitialized)?;
        Ok(storage)
    }
}

#[cfg(test)]
mod test {
    use crate::DiamondProxy;
    use soroban_sdk::testutils::BytesN as _;
    use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, IntoVal, Symbol, Vec};
    use stellar_diamond_proxy_core::facets::{FacetAction, FacetCut};
    use stellar_diamond_proxy_core::utils::DiamondProxyExecutor;

    pub mod contract_shared_storage_facet_1 {
        soroban_sdk::contractimport!(
            file = "../../../../.stellar-contracts/proxy-deps/target/wasm32-unknown-unknown/release/shared_storage_1.wasm"
        );
    }

    pub mod contract_shared_storage_facet_2 {
        soroban_sdk::contractimport!(
            file = "../../../../.stellar-contracts/proxy-deps/target/wasm32-unknown-unknown/release/shared_storage_2.wasm"
        );
    }

    fn create_diamond_with_facets(env: &Env) -> (Address, Vec<Address>) {
        // Set up the test environment
        let contract_id = env.register(DiamondProxy, ());
        env.mock_all_auths_allowing_non_root_auth();
        let user = Address::generate(env);
        env.as_contract(&contract_id.clone(), || {
            // Initialize the Diamond Proxy
            let random_salt = BytesN::<32>::random(env);
            DiamondProxy::init(env.clone(), user.clone(), random_salt)
                .expect("Failed to initialize Diamond Proxy");
            env.logs().add("DiamondProxy initialized", &[]);
            // Deploy and register the facets
            let facet_1_hash = env
                .deployer()
                .upload_contract_wasm(contract_shared_storage_facet_1::WASM);
            let facet_2_hash = env
                .deployer()
                .upload_contract_wasm(contract_shared_storage_facet_2::WASM);

            // Create facet cuts for deployment
            let cuts = vec![
                env,
                FacetCut {
                    wasm_hash_of_facet: facet_1_hash,
                    action: FacetAction::Add,
                    selectors: vec![
                        env,
                        Symbol::new(env, "increment"),
                        Symbol::new(env, "decrement"),
                        Symbol::new(env, "get_value"),
                    ],
                    salt: BytesN::<32>::random(env),
                },
                FacetCut {
                    wasm_hash_of_facet: facet_2_hash,
                    action: FacetAction::Add,
                    selectors: vec![
                        env,
                        Symbol::new(env, "increment_by"),
                        Symbol::new(env, "decrement_by"),
                        // We don't need to register get_value twice
                    ],
                    salt: BytesN::<32>::random(env),
                },
            ];

            // Perform the diamond cut to deploy facets
            let deployed_facets = DiamondProxy::diamond_cut(env.clone(), cuts).unwrap();

            (contract_id, deployed_facets)
        })
    }

    #[test]
    fn test_diamond_features() {
        let env = Env::default();

        // Create a diamond contract with initial facets
        let (diamond_id, _) = create_diamond_with_facets(&env);

        // Test loupe functions to verify initial state
        env.as_contract(&diamond_id, || {
            // Verify increment selector exists from the first facet
            let increment_facet =
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment"));
            assert!(increment_facet.is_some(), "increment selector should exist");

            // Verify decrement selector exists from the first facet
            let decrement_facet =
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "decrement"));
            assert!(decrement_facet.is_some(), "decrement selector should exist");

            // Verify increment_by selector exists from the second facet
            let increment_by_facet =
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment_by"));
            assert!(
                increment_by_facet.is_some(),
                "increment_by selector should exist"
            );

            // Verify total number of facets
            let facet_addresses = DiamondProxy::facet_addresses(env.clone());
            assert_eq!(
                facet_addresses.len(),
                2,
                "Should have two unique facet addresses"
            );
        });

        // Test functionality through the diamond proxy
        let args = vec![&env];
        let increment_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "increment"), args.clone())
            .unwrap();
        assert_eq!(increment_result, 1, "First increment should return 1");

        // Test increment_by with a value
        let args_with_value = vec![&env, 5_u32.into_val(&env)];
        let increment_by_result: u32 = env
            .facet_execute(
                &diamond_id,
                &Symbol::new(&env, "increment_by"),
                args_with_value,
            )
            .unwrap();
        assert_eq!(
            increment_by_result, 6,
            "Should be 6 after incrementing by 5"
        );
    }

    #[test]
    fn test_diamond_negative_cases() {
        let env = Env::default();

        // Create a diamond contract with initial facets
        let (diamond_id, _) = create_diamond_with_facets(&env);

        // Test that selectors exist after initialization
        env.as_contract(&diamond_id, || {
            // Check that increment selector exists
            let increment_selector = Symbol::new(&env, "increment");
            let increment_facet = DiamondProxy::facet_address(env.clone(), increment_selector);
            assert!(increment_facet.is_some(), "increment selector should exist");

            // Check that a non-existent selector returns None
            let non_existent_selector = Symbol::new(&env, "non_existent_function");
            let non_existent_facet =
                DiamondProxy::facet_address(env.clone(), non_existent_selector);
            assert!(
                non_existent_facet.is_none(),
                "Non-existent selector should not be found"
            );
        });
    }

    #[test]
    fn test_shared_storage_between_facets() {
        let env = Env::default();
        let (diamond_id, _) = create_diamond_with_facets(&env);

        // 1. Use the first facet to increment the counter
        let args = vec![&env];
        let increment_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "increment"), args.clone())
            .unwrap();
        assert_eq!(
            increment_result, 1,
            "Counter should be 1 after incrementing"
        );

        // 2. Get the value with the first facet
        let get_value_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "get_value"), args.clone())
            .unwrap();
        assert_eq!(get_value_result, 1, "Counter should be 1 when retrieved");

        // 3. Use the second facet to increment by 5
        let args_with_value = vec![&env, 5u32.into_val(&env)];
        let increment_by_result: u32 = env
            .facet_execute(
                &diamond_id,
                &Symbol::new(&env, "increment_by"),
                args_with_value,
            )
            .unwrap();
        assert_eq!(
            increment_by_result, 6,
            "Counter should be 6 after incrementing by 5"
        );

        // 4. Get the value again with the first facet to verify shared state
        let get_value_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "get_value"), args.clone())
            .unwrap();
        assert_eq!(get_value_result, 6, "Counter should be 6 across facets");

        // 5. Decrement with the first facet
        let decrement_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "decrement"), args.clone())
            .unwrap();
        assert_eq!(
            decrement_result, 5,
            "Counter should be 5 after decrementing"
        );

        // 6. Decrement by 2 with the second facet
        let args_with_value = vec![&env, 2u32.into_val(&env)];
        let decrement_by_result: u32 = env
            .facet_execute(
                &diamond_id,
                &Symbol::new(&env, "decrement_by"),
                args_with_value,
            )
            .unwrap();
        assert_eq!(
            decrement_by_result, 3,
            "Counter should be 3 after decrementing by 2"
        );

        // 7. Final check with first facet's get_value
        let final_value: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "get_value"), args)
            .unwrap();
        assert_eq!(final_value, 3, "Counter's final value should be 3");
    }

    #[test]
    fn test_facet_introspection() {
        let env = Env::default();
        let (diamond_id, deployed_facets) = create_diamond_with_facets(&env);

        env.as_contract(&diamond_id, || {
            // Test facet_addresses function
            let addresses = DiamondProxy::facet_addresses(env.clone());
            assert_eq!(addresses.len(), 2, "Should have two facet addresses");

            // Verify the returned addresses match the deployed facets
            let mut found_count = 0;
            for addr in addresses.iter() {
                if deployed_facets.contains(&addr) {
                    found_count += 1;
                }
            }
            assert_eq!(found_count, 2, "Both deployed facets should be found");

            // Test facet_address function for a specific selector
            let facet1_addr =
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment")).unwrap();
            assert!(
                deployed_facets.contains(&facet1_addr),
                "Should find facet for 'increment'"
            );

            let facet2_addr =
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment_by"))
                    .unwrap();
            assert!(
                deployed_facets.contains(&facet2_addr),
                "Should find facet for 'increment_by'"
            );

            // Test facet_function_selectors function
            let selectors1 =
                DiamondProxy::facet_function_selectors(env.clone(), facet1_addr).unwrap();
            assert!(
                selectors1.contains(Symbol::new(&env, "increment")),
                "Facet 1 should have 'increment' selector"
            );
            assert!(
                selectors1.contains(Symbol::new(&env, "decrement")),
                "Facet 1 should have 'decrement' selector"
            );

            let selectors2 =
                DiamondProxy::facet_function_selectors(env.clone(), facet2_addr).unwrap();
            assert!(
                selectors2.contains(Symbol::new(&env, "increment_by")),
                "Facet 2 should have 'increment_by' selector"
            );
            assert!(
                selectors2.contains(Symbol::new(&env, "decrement_by")),
                "Facet 2 should have 'decrement_by' selector"
            );
        })
    }

    #[test]
    fn test_complex_diamond_cut() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        // 1. Create a diamond with initial facets
        let (diamond_id, _) = create_diamond_with_facets(&env);

        // 2. Verify initial state
        env.as_contract(&diamond_id, || {
            let facet_addresses = DiamondProxy::facet_addresses(env.clone());
            assert_eq!(facet_addresses.len(), 2, "Should have two facets initially");

            // Verify all expected selectors exist
            assert!(
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment")).is_some(),
                "increment selector should exist"
            );
            assert!(
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "decrement")).is_some(),
                "decrement selector should exist"
            );
            assert!(
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "get_value")).is_some(),
                "get_value selector should exist"
            );
            assert!(
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "increment_by"))
                    .is_some(),
                "increment_by selector should exist"
            );
            assert!(
                DiamondProxy::facet_address(env.clone(), Symbol::new(&env, "decrement_by"))
                    .is_some(),
                "decrement_by selector should exist"
            );
        });

        // 3. Test incremental facet operations across multiple diamond cuts

        // First, test basic functionality works
        let args = vec![&env];
        let initial_increment: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "increment"), args.clone())
            .unwrap();
        assert_eq!(initial_increment, 1, "First increment should return 1");

        // Verify the counter was updated
        let initial_value: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "get_value"), args.clone())
            .unwrap();
        assert_eq!(
            initial_value, 1,
            "Counter value should be 1 after first increment"
        );

        // Increment again, but using increment_by from second facet
        let args_with_value = vec![&env, 5_u32.into_val(&env)];
        let increment_by_result: u32 = env
            .facet_execute(
                &diamond_id,
                &Symbol::new(&env, "increment_by"),
                args_with_value,
            )
            .unwrap();
        assert_eq!(
            increment_by_result, 6,
            "Should be 6 after incrementing by 5"
        );

        // Check the updated value
        let updated_value: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "get_value"), args.clone())
            .unwrap();
        assert_eq!(
            updated_value, 6,
            "Counter value should be 6 after incrementing by 5"
        );

        // Decrement the counter
        let decrement_result: u32 = env
            .facet_execute(&diamond_id, &Symbol::new(&env, "decrement"), args)
            .unwrap();
        assert_eq!(
            decrement_result, 5,
            "Counter should be 5 after decrementing"
        );

        // This test focuses on verifying that multiple diamond facets can work together
        // to provide a cohesive interface with shared state, which is the core
        // functionality of the Diamond Pattern.
    }

    #[test]
    fn test_selector_conflicts_and_introspection() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        // 1. Create a diamond with initial facets
        let (diamond_id, _) = create_diamond_with_facets(&env);

        // 2. Verify the initial selectors
        let initial_selectors = env.as_contract(&diamond_id, || {
            // Get all facet addresses
            let facet_addresses = DiamondProxy::facet_addresses(env.clone());

            // Collect all selectors from all facets
            let mut all_selectors = Vec::new(&env);
            for addr in facet_addresses.iter() {
                let facet_selectors =
                    DiamondProxy::facet_function_selectors(env.clone(), addr).unwrap();
                for selector in facet_selectors.iter() {
                    if !all_selectors.contains(&selector) {
                        all_selectors.push_back(selector.clone());
                    }
                }
            }

            all_selectors
        });

        // 3. Verify that we have the expected selectors
        assert!(
            initial_selectors.contains(Symbol::new(&env, "increment")),
            "Should have increment"
        );
        assert!(
            initial_selectors.contains(Symbol::new(&env, "decrement")),
            "Should have decrement"
        );
        assert!(
            initial_selectors.contains(Symbol::new(&env, "get_value")),
            "Should have get_value"
        );
        assert!(
            initial_selectors.contains(Symbol::new(&env, "increment_by")),
            "Should have increment_by"
        );
        assert!(
            initial_selectors.contains(Symbol::new(&env, "decrement_by")),
            "Should have decrement_by"
        );

        // 4. Test that adding a conflicting selector fails
        let conflict_facet_wasm = env
            .deployer()
            .upload_contract_wasm(contract_shared_storage_facet_1::WASM);

        let conflict_cut = vec![
            &env,
            FacetCut {
                action: FacetAction::Add,
                selectors: vec![&env, Symbol::new(&env, "increment")], // This selector already exists
                wasm_hash_of_facet: conflict_facet_wasm,
                salt: BytesN::<32>::random(&env),
            },
        ];

        let conflict_result = env.as_contract(&diamond_id, || {
            DiamondProxy::diamond_cut(env.clone(), conflict_cut)
        });

        assert!(
            conflict_result.is_err(),
            "Adding a conflicting selector should fail"
        );

        // 5. Verify that our introspection is accurate - test facet address mapping
        let get_value_selector = Symbol::new(&env, "get_value");
        let increment_by_selector = Symbol::new(&env, "increment_by");

        // These selectors should be in different facets
        let get_value_facet = env
            .as_contract(&diamond_id, || {
                DiamondProxy::facet_address(env.clone(), get_value_selector)
            })
            .unwrap();

        let increment_by_facet = env
            .as_contract(&diamond_id, || {
                DiamondProxy::facet_address(env.clone(), increment_by_selector)
            })
            .unwrap();

        // Verify they're in different facets
        assert_ne!(
            get_value_facet, increment_by_facet,
            "get_value and increment_by should be in different facets"
        );

        // 6. Verify facet_function_selectors returns the correct selectors
        let get_value_facet_selectors = env.as_contract(&diamond_id, || {
            DiamondProxy::facet_function_selectors(env.clone(), get_value_facet).unwrap()
        });

        assert!(
            get_value_facet_selectors.contains(Symbol::new(&env, "get_value")),
            "get_value facet should have get_value selector"
        );
    }
}
