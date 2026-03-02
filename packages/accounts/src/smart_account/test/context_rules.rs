extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractContext, ContractExecutable, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events, Ledger},
    vec, Address, Bytes, BytesN, Env, Map, String, Symbol, TryFromVal, Val, Vec,
};

use crate::{
    policies::Policy,
    smart_account::{
        storage::{
            add_context_rule, authenticate, contains_canonical_duplicate, do_check_auth,
            get_authenticated_signers, get_context_rule, get_context_rules_count,
            get_validated_context_by_id, remove_context_rule, update_context_rule_name,
            update_context_rule_valid_until, AuthPayload, ContextRule, ContextRuleType, Signer,
            SmartAccountStorageKey,
        },
        MAX_EXTERNAL_KEY_SIZE,
    },
};

#[contract]
struct MockContract;

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn enforce(
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(_e: &Env, _param: Val, _rule: ContextRule, _smart_account: Address) {}

    fn uninstall(e: &Env, _rule: ContextRule, _smart_account: Address) {
        let block_uninstall = e.storage().persistent().get(&symbol_short!("veto")).unwrap_or(false);
        if block_uninstall {
            panic!("Veto Uninstall Policy")
        }
    }
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        e.storage().persistent().get(&symbol_short!("verify")).unwrap_or(true)
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        Bytes::try_from_val(e, &key_data).unwrap()
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
    }
}

fn create_test_signers(e: &Env) -> Vec<Signer> {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);

    Vec::from_array(e, [Signer::Delegated(addr1), Signer::Delegated(addr2)])
}

fn create_test_policies(e: &Env) -> Vec<Address> {
    let policy1 = e.register(MockPolicyContract, ());
    let policy2 = e.register(MockPolicyContract, ());

    Vec::from_array(e, [policy1, policy2])
}

fn create_test_policies_map(e: &Env) -> Map<Address, Val> {
    let policies = create_test_policies(e);
    let mut policies_map = Map::new(e);
    for policy in policies.iter() {
        policies_map.set(policy, Val::from_void().into());
    }
    policies_map
}

fn setup_test_rule(e: &Env, address: &Address) -> ContextRule {
    e.as_contract(address, || {
        let contract_addr = Address::generate(e);

        add_context_rule(
            e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(e, "test_rule"),
            None,
            &create_test_signers(e),
            &Map::new(e),
        )
    })
}

fn get_context(contract: Address, fn_name: Symbol, args: Vec<Val>) -> Context {
    Context::Contract(ContractContext { contract, fn_name, args })
}

fn create_signatures(e: &Env, signers: &Vec<Signer>, context_rule_ids: Vec<u32>) -> AuthPayload {
    let mut signature_map = Map::new(e);
    for signer in signers.iter() {
        signature_map.set(signer, Bytes::new(e));
    }
    AuthPayload { signers: signature_map, context_rule_ids }
}

#[test]
fn do_check_auth_single_context_with_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.mock_all_auths();

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule with policies
        let policies_map = create_test_policies_map(&e);
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "policy_rule"),
            None,
            &Vec::new(&e),
            &policies_map,
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);
        let signatures = create_signatures(&e, &Vec::new(&e), vec![&e, rule.id]);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let result = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);

        assert!(result.is_ok());
    });
}

#[test]
fn do_check_auth_multiple_contexts_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.mock_all_auths();

    e.as_contract(&address, || {
        let contract_addr1 = Address::generate(&e);
        let contract_addr2 = Address::generate(&e);
        let context_type1 = ContextRuleType::CallContract(contract_addr1.clone());
        let context_type2 = ContextRuleType::CallContract(contract_addr2.clone());

        // Add rules for both contexts
        let signers = create_test_signers(&e);
        let policies = Map::new(&e);
        let rule1 = add_context_rule(
            &e,
            &context_type1,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &policies,
        );
        let rule2 = add_context_rule(
            &e,
            &context_type2,
            &String::from_str(&e, "rule2"),
            None,
            &signers,
            &policies,
        );

        let context1 = get_context(contract_addr1, symbol_short!("test1"), vec![&e]);
        let context2 = get_context(contract_addr2, symbol_short!("test2"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context1, context2]);

        // Create signatures with all required signers
        let mut all_signers = rule1.signers.clone();
        all_signers.append(&rule2.signers);
        let signatures = create_signatures(&e, &all_signers, vec![&e, rule1.id, rule2.id]);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let result = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);

        assert!(result.is_ok());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3003)")]
fn do_check_auth_authentication_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier_addr = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Create rule with external signer
        let key_data = Bytes::from_array(&e, &[1, 2, 3, 4]);
        let external_signer = Signer::External(verifier_addr.clone(), key_data.clone());
        let signers = Vec::from_array(&e, [external_signer.clone()]);
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "external_rule"),
            None,
            &signers,
            &Map::new(&e),
        );

        // Set verifier to return false
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &false);
        });

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);

        let mut signature_map = Map::new(&e);
        signature_map.set(external_signer, Bytes::from_array(&e, &[5, 6, 7, 8]));
        let signatures =
            AuthPayload { signers: signature_map, context_rule_ids: vec![&e, rule.id] };
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let _ = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn do_check_auth_context_validation_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.mock_all_auths();

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule requiring 2 signers
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "strict_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);

        // Provide insufficient signers
        let insufficient_signers = rule.signers.slice(..1);
        let signatures = create_signatures(&e, &insufficient_signers, vec![&e, rule.id]);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let _ = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3014)")]
fn do_check_auth_context_rule_ids_length_mismatch_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        let signers = create_test_signers(&e);
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule"),
            None,
            &signers,
            &Map::new(&e),
        );

        let context_a = get_context(contract_addr.clone(), symbol_short!("fn_a"), vec![&e]);
        let context_b = get_context(contract_addr, symbol_short!("fn_b"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context_a, context_b]);

        let mut signature_map = Map::new(&e);
        for signer in signers.iter() {
            signature_map.set(signer, Bytes::new(&e));
        }
        // 2 auth contexts but only 1 rule ID — must fail.
        let signatures =
            AuthPayload { signers: signature_map, context_rule_ids: vec![&e, rule.id] };

        let payload = Bytes::from_array(&e, &[1u8; 32]);
        let _ = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);
    });
}

#[test]
fn add_context_rule_multiple_rules() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies_map = create_test_policies_map(&e);

        let rule1 = add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "rule_1"),
            Some(1000),
            &signers,
            &Map::new(&e),
        );

        let rule2 = add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "rule_2"),
            None,
            &signers,
            &policies_map,
        );

        assert_eq!(rule1.id, 0);
        assert_eq!(rule1.name, String::from_str(&e, "rule_1"));
        assert_eq!(rule1.signers.len(), 2);
        assert_eq!(rule1.policies.len(), 0);
        assert_eq!(rule1.valid_until, Some(1000));

        assert_eq!(rule2.id, 1);
        assert_eq!(rule2.name, String::from_str(&e, "rule_2"));
        assert_eq!(rule2.signers.len(), 2);
        assert_eq!(rule2.policies.len(), 2);
        assert_eq!(rule2.valid_until, None);

        // Events: 2 ContextRuleAdded + 2 SignerRegistered (shared) + 2 PolicyRegistered
        // = 6
        assert_eq!(e.events().all().events().len(), 6);
        assert_eq!(get_context_rules_count(&e), 2);
    });
}

#[test]
fn add_context_rule_different_context_types() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let contract_addr = Address::generate(&e);
        let wasm_hash = BytesN::from_array(&e, &[1u8; 32]);

        let call_rule = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(&e, "call_rule"),
            None,
            &signers,
            &Map::new(&e),
        );

        let create_rule = add_context_rule(
            &e,
            &ContextRuleType::CreateContract(wasm_hash),
            &String::from_str(&e, "create_rule"),
            None,
            &signers,
            &Map::new(&e),
        );

        assert_eq!(call_rule.id, 0);
        assert_eq!(create_rule.id, 1);
        assert!(matches!(call_rule.context_type, ContextRuleType::CallContract(_)));
        assert!(matches!(create_rule.context_type, ContextRuleType::CreateContract(_)));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3005)")]
fn add_context_rule_past_valid_until_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let contract_addr = Address::generate(&e);
        e.ledger().set_sequence_number(100);

        add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(&e, "expired_rule"),
            Some(99),
            &signers,
            &Map::new(&e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3007)")]
fn add_context_rule_duplicate_signer_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr);
        let policies_map = create_test_policies_map(&e);

        // Create signers with duplicate
        let signer1 = Signer::Delegated(Address::generate(&e));
        let signer2 = Signer::Delegated(Address::generate(&e));
        let duplicate_signer = signer1.clone(); // Duplicate of signer1

        let mut signers = Vec::new(&e);
        signers.push_back(signer1);
        signers.push_back(signer2);
        signers.push_back(duplicate_signer); // This should cause the error

        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "test_rule"),
            None,
            &signers,
            &policies_map,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3015)")]
fn add_context_rule_name_too_long_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let contract_addr = Address::generate(&e);
        let too_long_name = String::from_str(&e, "name_that_is_way_too_long");

        add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr),
            &too_long_name,
            None,
            &signers,
            &Map::new(&e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3013)")]
fn add_context_rule_oversized_external_key_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let oversized_key = Bytes::from_array(&e, &[0u8; 257]);
        let signer = Signer::External(verifier.clone(), oversized_key);
        let signers = Vec::from_array(&e, [signer]);

        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "oversized_key_rule"),
            None,
            &signers,
            &Map::new(&e),
        );
    });
}

#[test]
fn add_context_rule_max_size_external_key_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let max_key = Bytes::from_slice(&e, &[0u8; 256]);
        let signer = Signer::External(verifier.clone(), max_key);
        let signers = Vec::from_array(&e, [signer]);

        let rule = add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "max_key_rule"),
            None,
            &signers,
            &Map::new(&e),
        );

        assert_eq!(rule.signers.len(), 1);
        if let Signer::External(_, key_data) = rule.signers.get(0).unwrap() {
            assert_eq!(key_data.len(), MAX_EXTERNAL_KEY_SIZE);
        }
    });
}

#[test]
fn update_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let future_sequence = e.ledger().sequence() + 500;
        // Update name and valid_until separately
        update_context_rule_name(&e, rule.id, &String::from_str(&e, "modified_rule"));
        update_context_rule_valid_until(&e, rule.id, Some(future_sequence));
        assert_eq!(e.events().all().events().len(), 2);

        let modified_rule = get_context_rule(&e, rule.id);

        assert_eq!(modified_rule.id, rule.id);
        assert_eq!(modified_rule.name, String::from_str(&e, "modified_rule"));
        assert_eq!(modified_rule.valid_until, Some(future_sequence));

        // Verify it was actually stored
        let rule = get_context_rule(&e, rule.id);
        assert_eq!(rule.name, String::from_str(&e, "modified_rule"));

        // Modify again new valid_until None
        update_context_rule_valid_until(&e, rule.id, None);
        let modified_rule = get_context_rule(&e, rule.id);

        assert_eq!(modified_rule.id, rule.id);
        assert_eq!(modified_rule.valid_until, None);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3015)")]
fn update_context_rule_name_too_long_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        update_context_rule_name(&e, rule.id, &String::from_str(&e, "name_that_is_way_too_long"));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn update_context_rule_nonexistent_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        update_context_rule_name(&e, 999, &String::from_str(&e, "nonexistent"));
        // Non-existent ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3005)")]
fn update_context_rule_past_valid_until_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        e.ledger().set_sequence_number(100);

        update_context_rule_valid_until(&e, rule.id, Some(99));
    });
}

#[test]
fn remove_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        // Verify rule exists
        let retrieved_rule = get_context_rule(&e, rule.id);
        assert_eq!(retrieved_rule.id, rule.id);

        remove_context_rule(&e, rule.id);
        // Events: 1 ContextRuleRemoved + 2 SignerDeregistered (no policies) = 3
        assert_eq!(e.events().all().events().len(), 3);
    });

    let rule = e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);

        add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(&e, "test_rule"),
            None,
            &create_test_signers(&e),
            &create_test_policies_map(&e),
        )
    });

    // `unistall` of the first policy panics
    e.as_contract(&rule.policies.get_unchecked(0), || {
        e.storage().persistent().set(&symbol_short!("veto"), &true);
    });

    // Removal succeeds
    e.as_contract(&address, || {
        remove_context_rule(&e, rule.id);
        // Events: 1 ContextRuleRemoved + 2 SignerDeregistered + 2 PolicyDeregistered =
        // 5
        assert_eq!(e.events().all().events().len(), 5);
        assert_eq!(get_context_rules_count(&e), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn remove_context_rule_nonexistent_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        remove_context_rule(&e, 999); // Non-existent ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3003)")]
fn authenticate_external_signer_verification_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier_addr = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let key_data = Bytes::from_array(&e, &[1, 2, 3, 4]);
        let sig_data = Bytes::from_array(&e, &[5, 6, 7, 8]);
        let signer = Signer::External(verifier_addr.clone(), key_data.clone());

        // Set verifier to return false
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &false);
        });

        let mut signature_map = Map::new(&e);
        signature_map.set(signer, sig_data);

        let payload = Bytes::from_array(&e, &[1u8; 32]);

        authenticate(&e, &e.crypto().sha256(&payload), &signature_map);
    });
}

#[test]
fn authenticate_mixed_signers_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier_addr = e.register(MockVerifierContract, ());

    e.mock_all_auths();

    e.as_contract(&address, || {
        let native_addr = Address::generate(&e);
        let key_data = Bytes::from_array(&e, &[1u8; 32]);

        let native_signer = Signer::Delegated(native_addr);
        let external_signer = Signer::External(verifier_addr.clone(), key_data);

        // Set verifier to return true
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &true);
        });

        let mut signature_map = Map::new(&e);
        signature_map.set(native_signer, Bytes::new(&e));
        signature_map.set(external_signer, Bytes::from_array(&e, &[5, 6, 7, 8]));

        let payload = Bytes::from_array(&e, &[1u8; 32]);

        authenticate(&e, &e.crypto().sha256(&payload), &signature_map);
    });
}

#[test]
fn get_authenticated_signers_combos() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // all match
        let rule_signers = create_test_signers(&e);
        let all_signers = rule_signers.clone();
        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);
        assert_eq!(authenticated.len(), 2);
        assert_eq!(authenticated, rule_signers);

        // empty all_signers
        let authenticated = get_authenticated_signers(&e, &rule_signers, &Vec::new(&e));
        assert_eq!(authenticated.len(), 0);

        // empty rule_signers
        let authenticated = get_authenticated_signers(&e, &Vec::new(&e), &create_test_signers(&e));
        assert_eq!(authenticated.len(), 0);

        // partial match
        let addr1 = Address::generate(&e);
        let addr2 = Address::generate(&e);
        let addr3 = Address::generate(&e);
        let rule_signers = Vec::from_array(
            &e,
            [Signer::Delegated(addr1.clone()), Signer::Delegated(addr2.clone())],
        );
        let all_signers = Vec::from_array(
            &e,
            [
                Signer::Delegated(addr1.clone()),
                Signer::Delegated(addr3.clone()), // addr2 is missing, addr3 is extra
            ],
        );
        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);
        assert_eq!(authenticated.len(), 1); // Only addr1 matches
        assert_eq!(authenticated.get(0).unwrap(), Signer::Delegated(addr1.clone()));

        // no match
        let rule_signers =
            Vec::from_array(&e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]);
        let all_signers = Vec::from_array(&e, [Signer::Delegated(addr3)]);
        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);
        assert_eq!(authenticated.len(), 0);
    });
}

#[test]
fn get_validated_context_by_id_direct_match_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "direct_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let (validated_rule, _, authenticated_signers) =
            get_validated_context_by_id(&e, &context, &rule.signers, rule.id);

        assert_eq!(validated_rule.id, rule.id);
        assert_eq!(authenticated_signers.len(), rule.signers.len());
    });
}

#[test]
fn get_validated_context_by_id_default_rule_matches_any_context() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);

        let rule = add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "default_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("call"), vec![&e]);
        let (validated_rule, _, authenticated_signers) =
            get_validated_context_by_id(&e, &context, &rule.signers, rule.id);

        assert_eq!(validated_rule.id, rule.id);
        assert_eq!(authenticated_signers.len(), rule.signers.len());
    });
}

#[test]
fn get_validated_context_by_id_with_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        let policies_map = create_test_policies_map(&e);
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "policy_rule"),
            None,
            &Vec::new(&e),
            &policies_map,
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let (validated_rule, _, _) =
            get_validated_context_by_id(&e, &context, &Vec::new(&e), rule.id);

        assert_eq!(validated_rule.id, rule.id);
    });
}

#[test]
fn get_validated_context_by_id_not_yet_expired_succeeds() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        e.ledger().set_sequence_number(50);

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "future_expiry_rule"),
            Some(100),
            &create_test_signers(&e),
            &Map::new(&e),
        );

        // Sequence 50 < valid_until 100 → not expired, should succeed
        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let (validated_rule, _, _) =
            get_validated_context_by_id(&e, &context, &rule.signers, rule.id);
        assert_eq!(validated_rule.id, rule.id);
    });
}

#[test]
fn get_validated_context_by_id_create_contract_host_fn_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let wasm_hash = BytesN::from_array(&e, &[7u8; 32]);
        let context_type = ContextRuleType::CreateContract(wasm_hash.clone());

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "create_contract_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            salt: BytesN::from_array(&e, &[1u8; 32]),
            executable: ContractExecutable::Wasm(wasm_hash),
        });

        let (validated_rule, _, _) =
            get_validated_context_by_id(&e, &context, &rule.signers, rule.id);
        assert_eq!(validated_rule.id, rule.id);
    });
}

#[test]
fn get_validated_context_by_id_create_contract_with_ctor_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let wasm_hash = BytesN::from_array(&e, &[8u8; 32]);
        let context_type = ContextRuleType::CreateContract(wasm_hash.clone());

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "ctor_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context =
            Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
                salt: BytesN::from_array(&e, &[2u8; 32]),
                executable: ContractExecutable::Wasm(wasm_hash),
                constructor_args: Vec::new(&e),
            });

        let (validated_rule, _, _) =
            get_validated_context_by_id(&e, &context, &rule.signers, rule.id);
        assert_eq!(validated_rule.id, rule.id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn get_validated_context_by_id_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);

        get_validated_context_by_id(&e, &context, &Vec::new(&e), 999u32);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_by_id_expired_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "expiring_rule"),
            Some(50),
            &create_test_signers(&e),
            &Map::new(&e),
        );

        e.ledger().set_sequence_number(100);

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        get_validated_context_by_id(&e, &context, &rule.signers, rule.id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_by_id_wrong_context_type_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_a = Address::generate(&e);
        let contract_b = Address::generate(&e);

        let rule = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_a),
            &String::from_str(&e, "contract_a_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_b, symbol_short!("test"), vec![&e]);
        get_validated_context_by_id(&e, &context, &rule.signers, rule.id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_by_id_insufficient_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "two_signer_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let only_one = rule.signers.slice(..1);
        get_validated_context_by_id(&e, &context, &only_one, rule.id);
    });
}

// ################## CANONICAL DUPLICATE DETECTION TESTS ##################

/// Mock verifier that canonicalizes keys by returning only the first 32
/// bytes. This simulates a verifier (like WebAuthn) where key_data contains
/// both a cryptographic key and additional metadata (like a credential ID).
#[contract]
struct MockCanonicalizingVerifier;

#[contractimpl]
impl MockCanonicalizingVerifier {
    pub fn verify(e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        e.storage().persistent().get(&symbol_short!("verify")).unwrap_or(true)
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        let key = Bytes::try_from_val(e, &key_data).unwrap();
        key.slice(0..32)
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(
            e,
            key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap().slice(0..32)),
        )
    }
}

#[test]
fn contains_canonical_duplicate_same_canonical_keys() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let verifier = e.register(MockCanonicalizingVerifier, ());

        let mut existing = Bytes::from_array(&e, &[1u8; 32]);
        existing.extend_from_array(&[0xAA; 8]);
        let signers = Vec::from_array(&e, [Signer::External(verifier.clone(), existing)]);

        let mut candidate = Bytes::from_array(&e, &[1u8; 32]);
        candidate.extend_from_array(&[0xBB; 8]);
        let new_signer = Signer::External(verifier.clone(), candidate);

        assert!(contains_canonical_duplicate(&e, &signers, &new_signer));
    });
}

#[test]
fn contains_canonical_duplicate_different_canonical_keys() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let verifier = e.register(MockCanonicalizingVerifier, ());

        let mut existing = Bytes::from_array(&e, &[1u8; 32]);
        existing.extend_from_array(&[0xAA; 8]);
        let signers = Vec::from_array(&e, [Signer::External(verifier.clone(), existing)]);

        let mut candidate = Bytes::from_array(&e, &[2u8; 32]);
        candidate.extend_from_array(&[0xBB; 8]);
        let new_signer = Signer::External(verifier.clone(), candidate);

        assert!(!contains_canonical_duplicate(&e, &signers, &new_signer));
    });
}

#[test]
fn contains_canonical_duplicate_no_matching_verifier() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let verifier1 = e.register(MockCanonicalizingVerifier, ());
        let verifier2 = e.register(MockCanonicalizingVerifier, ());

        let mut existing = Bytes::from_array(&e, &[3u8; 32]);
        existing.extend_from_array(&[0xAA; 8]);
        let signers = Vec::from_array(&e, [Signer::External(verifier1, existing)]);

        let mut candidate = Bytes::from_array(&e, &[3u8; 32]);
        candidate.extend_from_array(&[0xBB; 8]);
        let new_signer = Signer::External(verifier2, candidate);

        assert!(!contains_canonical_duplicate(&e, &signers, &new_signer));
    });
}

#[test]
fn contains_canonical_duplicate_delegated_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let delegated_a = Address::generate(&e);
        let delegated_b = Address::generate(&e);
        let signers = Vec::from_array(&e, [Signer::Delegated(delegated_a.clone())]);

        assert!(contains_canonical_duplicate(&e, &signers, &Signer::Delegated(delegated_a)));
        assert!(!contains_canonical_duplicate(&e, &signers, &Signer::Delegated(delegated_b)));
    });
}

#[test]
fn contains_canonical_duplicate_external_with_delegated_existing() {
    // When the new signer is External but the existing signer list contains a
    // Delegated signer, the `if let Signer::External` branch does not match
    // for the delegated entry — covering the implicit-else path.
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let verifier = e.register(MockCanonicalizingVerifier, ());
        let delegated_signer = Signer::Delegated(Address::generate(&e));
        let existing = Vec::from_array(&e, [delegated_signer]);

        let mut key_data = Bytes::from_array(&e, &[5u8; 32]);
        key_data.extend_from_array(&[0xCC; 8]);
        let new_signer = Signer::External(verifier, key_data);

        // No External signer with the same verifier in the list → no duplicate
        assert!(!contains_canonical_duplicate(&e, &existing, &new_signer));
    });
}

// ################## MATH OVERFLOW TESTS ##################

#[test]
#[should_panic(expected = "Error(Contract, #3012)")]
fn add_context_rule_next_id_overflow_fails() {
    // When NextId == u32::MAX, incrementing it after storing a new rule
    // must panic with MathOverflow rather than wrapping.
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        e.storage().instance().set(&SmartAccountStorageKey::NextId, &u32::MAX);

        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "overflow"),
            None,
            &Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]),
            &Map::new(&e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3012)")]
fn add_context_rule_next_signer_id_overflow_fails() {
    // When NextSignerId == u32::MAX, registering a new signer must panic
    // with MathOverflow rather than wrapping.
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        e.storage().instance().set(&SmartAccountStorageKey::NextSignerId, &u32::MAX);

        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "overflow"),
            None,
            &Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]),
            &Map::new(&e),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3012)")]
fn add_context_rule_next_policy_id_overflow_fails() {
    // When NextPolicyId == u32::MAX, registering a new policy must panic
    // with MathOverflow rather than wrapping.
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        e.storage().instance().set(&SmartAccountStorageKey::NextPolicyId, &u32::MAX);

        let policy = e.register(MockPolicyContract, ());
        let mut policies = Map::new(&e);
        policies.set(policy, Val::from_void().into());

        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "overflow"),
            None,
            &Vec::new(&e),
            &policies,
        );
    });
}
