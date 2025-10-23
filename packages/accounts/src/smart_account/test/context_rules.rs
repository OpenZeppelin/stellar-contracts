extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractContext, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events, Ledger},
    vec, Address, Bytes, BytesN, Env, Map, String, Symbol, Val, Vec,
};

use crate::{
    policies::Policy,
    smart_account::{
        get_context_rules, get_validated_context,
        storage::{
            add_context_rule, authenticate, can_enforce_all_policies, do_check_auth,
            get_authenticated_signers, get_context_rule, get_valid_context_rules,
            remove_context_rule, update_context_rule_name, update_context_rule_valid_until,
            ContextRule, ContextRuleType, Signatures, Signer,
        },
        MAX_CONTEXT_RULES,
    },
};

#[contract]
struct MockContract;

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn can_enforce(
        e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) -> bool {
        e.storage().persistent().get(&symbol_short!("enforce")).unwrap_or(true)
    }

    fn enforce(
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(_e: &Env, _param: Val, _rule: ContextRule, _smart_account: Address) {}

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        e.storage().persistent().get(&symbol_short!("verify")).unwrap_or(true)
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

fn create_signatures(e: &Env, signers: &Vec<Signer>) -> Signatures {
    let mut signature_map = Map::new(e);
    for signer in signers.iter() {
        signature_map.set(signer, Bytes::new(e));
    }
    Signatures(signature_map)
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
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "policy_rule"),
            None,
            &Vec::new(&e),
            &policies_map,
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);
        let signatures = create_signatures(&e, &Vec::new(&e));
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
        let signatures = create_signatures(&e, &all_signers);
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
        add_context_rule(
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
        let signatures = Signatures(signature_map);
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
        let signatures = create_signatures(&e, &insufficient_signers);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let _ = do_check_auth(&e, &e.crypto().sha256(&payload), &signatures, &auth_contexts);
    });
}
#[test]
fn add_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let contract_addr = Address::generate(&e);
        let future_sequence = e.ledger().sequence() + 1000;

        let rule = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(&e, "test_rule"),
            Some(future_sequence),
            &signers,
            &Map::new(&e),
        );

        assert_eq!(rule.id, 0);
        assert_eq!(rule.name, String::from_str(&e, "test_rule"));
        assert_eq!(rule.signers.len(), 2);
        assert_eq!(rule.policies.len(), 0);
        assert_eq!(rule.valid_until, Some(future_sequence));
        assert_eq!(e.events().all().len(), 1);
    });
}

#[test]
fn add_context_rule_multiple_rules_increment_id() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let contract_addr1 = Address::generate(&e);
        let contract_addr2 = Address::generate(&e);

        let rule1 = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr1),
            &String::from_str(&e, "rule_1"),
            None,
            &signers,
            &Map::new(&e),
        );

        let rule2 = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr2),
            &String::from_str(&e, "rule_2"),
            Some(1000),
            &signers,
            &Map::new(&e),
        );

        assert_eq!(rule1.id, 0);
        assert_eq!(rule2.id, 1);
        assert_eq!(rule1.name, String::from_str(&e, "rule_1"));
        assert_eq!(rule2.name, String::from_str(&e, "rule_2"));
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
        //let current = e.ledger().sequence();
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
fn update_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let future_sequence = e.ledger().sequence() + 500;
        // Update name and valid_until separately
        update_context_rule_name(&e, rule.id, &String::from_str(&e, "modified_rule"));
        update_context_rule_valid_until(&e, rule.id, Some(future_sequence));
        assert_eq!(e.events().all().len(), 2);

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

        // Remove the rule
        remove_context_rule(&e, rule.id);
        assert_eq!(e.events().all().len(), 1);
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
fn can_enforce_all_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let mut rule = setup_test_rule(&e, &address);
        rule.policies = create_test_policies(&e);
        let matched_signers = rule.signers.clone();
        let context = get_context(Address::generate(&e), symbol_short!("test"), vec![&e]);

        let result = can_enforce_all_policies(&e, &context, &rule, &matched_signers);
        assert!(result);
    });
}

#[test]
fn can_enforce_all_policies_one_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let mut rule = setup_test_rule(&e, &address);
        rule.policies = create_test_policies(&e);
        let matched_signers = rule.signers.clone();
        let context = get_context(Address::generate(&e), symbol_short!("test"), vec![&e]);

        // Set first policy to return true, second to return false
        e.as_contract(&rule.policies.get(0).unwrap(), || {
            e.storage().persistent().set(&symbol_short!("enforce"), &true);
        });
        e.as_contract(&rule.policies.get(1).unwrap(), || {
            e.storage().persistent().set(&symbol_short!("enforce"), &false);
        });

        let result = can_enforce_all_policies(&e, &context, &rule, &matched_signers);
        assert!(!result); // Should fail because one policy returns false
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
fn get_authenticated_signers_all_match() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let rule_signers = create_test_signers(&e);
        let all_signers = rule_signers.clone();

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 2);
        assert_eq!(authenticated, rule_signers);
    });
}

#[test]
fn get_authenticated_signers_partial_match() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
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
                Signer::Delegated(addr3), // addr2 is missing, addr3 is extra
            ],
        );

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 1); // Only addr1 matches
        assert_eq!(authenticated.get(0).unwrap(), Signer::Delegated(addr1));
    });
}

#[test]
fn get_authenticated_signers_no_match() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let addr1 = Address::generate(&e);
        let addr2 = Address::generate(&e);
        let addr3 = Address::generate(&e);
        let addr4 = Address::generate(&e);

        let rule_signers =
            Vec::from_array(&e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]);

        let all_signers = Vec::from_array(&e, [Signer::Delegated(addr3), Signer::Delegated(addr4)]);

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 0);
    });
}

#[test]
fn get_authenticated_signers_empty_rule_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let rule_signers = Vec::new(&e);
        let all_signers = create_test_signers(&e);

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 0);
    });
}

#[test]
fn get_authenticated_signers_empty_all_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let rule_signers = create_test_signers(&e);
        let all_signers = Vec::new(&e);

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 0);
    });
}

#[test]
fn get_valid_context_rules_matched_rules_only() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add two rules for the same context type
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule2"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let valid_rules = get_valid_context_rules(&e, &context_type);

        // Should return both rules in reverse order (last added first)
        assert_eq!(valid_rules.len(), 2);
        assert_eq!(valid_rules.get(0).unwrap().name, String::from_str(&e, "rule2"));
        assert_eq!(valid_rules.get(1).unwrap().name, String::from_str(&e, "rule1"));
    });
}

#[test]
fn get_valid_context_rules_with_defaults() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add a matched rule
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "matched"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        // Add default rules
        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "default"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let valid_rules = get_valid_context_rules(&e, &context_type);

        // Should return matched rule first, then defaults
        assert_eq!(valid_rules.len(), 2);
        assert_eq!(valid_rules.get(0).unwrap().name, String::from_str(&e, "matched"));
        assert_eq!(valid_rules.get(1).unwrap().name, String::from_str(&e, "default"));
    });
}

#[test]
fn get_valid_context_rules_only_defaults() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr);

        // Add only default rules with different signers to ensure different
        // fingerprints
        let signers = create_test_signers(&e);
        let addr3 = Address::generate(&e);
        let signers2 = Vec::from_array(&e, [Signer::Delegated(addr3)]);

        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "default1"),
            None,
            &signers,
            &Map::new(&e),
        );
        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "default2"),
            Some(1000),
            &signers2,
            &Map::new(&e),
        );

        let valid_rules = get_valid_context_rules(&e, &context_type);

        // Should return only defaults in reverse order
        assert_eq!(valid_rules.len(), 2);
        assert_eq!(valid_rules.get(0).unwrap().name, String::from_str(&e, "default2"));
        assert_eq!(valid_rules.get(1).unwrap().name, String::from_str(&e, "default1"));
    });
}

#[test]
fn get_valid_context_rules_filters_expired() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Set current ledger sequence
        e.ledger().set_sequence_number(100);

        // Add valid rule (future expiration)
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "valid"),
            Some(200),
            &create_test_signers(&e),
            &Map::new(&e),
        );

        // Add expired rule
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "expired"),
            Some(150),
            &create_test_signers(&e),
            &Map::new(&e),
        );

        // Forward ledger sequence
        e.ledger().set_sequence_number(160);

        // Add rule with no expiration
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "no_expiry"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let valid_rules = get_valid_context_rules(&e, &context_type);

        // Should only return non-expired rules
        assert_eq!(valid_rules.len(), 2);
        assert_eq!(valid_rules.get(0).unwrap().name, String::from_str(&e, "no_expiry"));
        assert_eq!(valid_rules.get(1).unwrap().name, String::from_str(&e, "valid"));
    });
}

#[test]
fn get_valid_context_rules_empty_result() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr);

        let valid_rules = get_valid_context_rules(&e, &context_type);

        // Should return empty vector when no rules exist
        assert_eq!(valid_rules.len(), 0);
    });
}

#[test]
fn get_validated_context_call_contract_with_signers_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule with signers
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "test_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);

        let (validated_rule, _, authenticated_signers) =
            get_validated_context(&e, &context, &rule.signers);

        assert_eq!(validated_rule.id, rule.id);
        assert_eq!(authenticated_signers.len(), 2);
        assert_eq!(authenticated_signers, rule.signers);
    });
}

#[test]
fn get_validated_context_create_contract_with_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let wasm_hash = BytesN::from_array(&e, &[1u8; 32]);
        let context_type = ContextRuleType::CreateContract(wasm_hash.clone());

        // Add rule with policies
        let policies_map = create_test_policies_map(&e);
        let empty_signers = Vec::new(&e);
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "policy_rule"),
            None,
            &empty_signers,
            &policies_map,
        );

        let context =
            Context::CreateContractWithCtorHostFn(CreateContractWithConstructorHostFnContext {
                executable: soroban_sdk::auth::ContractExecutable::Wasm(wasm_hash),
                salt: BytesN::from_array(&e, &[2u8; 32]),
                constructor_args: Vec::new(&e),
            });

        let (validated_rule, _, authenticated_signers) =
            get_validated_context(&e, &context, &Vec::new(&e));

        assert_eq!(validated_rule.id, rule.id);
        assert_eq!(authenticated_signers.len(), 0);
    });
}

#[test]
fn get_validated_context_create_contract_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let wasm_hash = BytesN::from_array(&e, &[1u8; 32]);
        let context_type = ContextRuleType::CreateContract(wasm_hash.clone());

        // Add rule for contract creation
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "create_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            executable: soroban_sdk::auth::ContractExecutable::Wasm(wasm_hash),
            salt: BytesN::from_array(&e, &[2u8; 32]),
        });

        let (validated_rule, _, authenticated_signers) =
            get_validated_context(&e, &context, &rule.signers);

        assert_eq!(validated_rule.id, rule.id);
        assert_eq!(authenticated_signers.len(), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_call_contract_with_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule with policies
        let policies_vec = create_test_policies(&e);
        let mut policies_map = Map::new(&e);
        for policy in policies_vec.iter() {
            policies_map.set(policy, Val::from_void().into());
        }
        let context_val_name = String::from_str(&e, "policy_rule");
        let empty_signers = Vec::new(&e);
        add_context_rule(&e, &context_type, &context_val_name, None, &empty_signers, &policies_map);
        let failing_policy = policies_vec.get(1).unwrap();
        e.as_contract(&failing_policy, || {
            e.storage().persistent().set(&symbol_short!("enforce"), &false);
        });

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        get_validated_context(&e, &context, &Vec::new(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_insufficient_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

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
        let signers = rule.signers.clone();
        let insufficient_signers = signers.slice(..1); // Only 1 signer

        get_validated_context(&e, &context, &insufficient_signers);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3002)")]
fn get_validated_context_no_matching_rules_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let different_addr = Address::generate(&e);

        // Add rule for different contract
        let different_context_type = ContextRuleType::CallContract(different_addr);
        add_context_rule(
            &e,
            &different_context_type,
            &String::from_str(&e, "other_rule"),
            None,
            &create_test_signers(&e),
            &Map::new(&e),
        );

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let all_signers = create_test_signers(&e);

        get_validated_context(&e, &context, &all_signers);
    });
}

#[test]
fn get_context_rules_multiple_rules() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add multiple rules with different signers to ensure different fingerprints
        let signers = create_test_signers(&e);
        let addr3 = Address::generate(&e);
        let addr4 = Address::generate(&e);
        let signers2 = Vec::from_array(&e, [Signer::Delegated(addr3)]);
        let signers3 = Vec::from_array(&e, [Signer::Delegated(addr4)]);

        let rule1 = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers.clone(),
            &Map::new(&e),
        );
        let rule2 = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule2"),
            Some(1000),
            &signers2.clone(),
            &Map::new(&e),
        );
        let rule3 = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule3"),
            Some(2000),
            &signers3.clone(),
            &Map::new(&e),
        );

        let rules = get_context_rules(&e, &context_type);

        assert_eq!(rules.len(), 3);
        // Rules are returned in order they were added
        assert_eq!(rules.get(0).unwrap().id, rule1.id);
        assert_eq!(rules.get(1).unwrap().id, rule2.id);
        assert_eq!(rules.get(2).unwrap().id, rule3.id);
    });
}

#[test]
fn get_context_rules_empty_result() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr);

        let rules = get_context_rules(&e, &context_type);

        // Should return empty vector when no rules exist
        assert_eq!(rules.len(), 0);
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
#[should_panic(expected = "Error(Contract, #3012)")]
fn add_context_rule_too_many_rules_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies_map = Map::new(&e);

        // Add MAX_CONTEXT_RULES (15) rules successfully
        for i in 0..MAX_CONTEXT_RULES {
            let contract_addr = Address::generate(&e);
            let context_type = ContextRuleType::CallContract(contract_addr);

            add_context_rule(
                &e,
                &context_type,
                &String::from_str(&e, "test_rule"),
                Some(i),
                &signers,
                &policies_map,
            );
        }

        // Try to add the 16th rule - this should fail
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr);

        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule_16"),
            None,
            &signers,
            &policies_map,
        );
    });
}

#[test]
fn add_context_rule_count_allows_reuse_after_removal() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies_map = Map::new(&e);

        // Add MAX_CONTEXT_RULES (15) rules successfully
        let mut rule_ids = Vec::new(&e);
        for i in 0..MAX_CONTEXT_RULES {
            let contract_addr = Address::generate(&e);
            let context_type = ContextRuleType::CallContract(contract_addr);

            let rule = add_context_rule(
                &e,
                &context_type,
                &String::from_str(&e, "test_rule"),
                Some(i + 100),
                &signers,
                &policies_map,
            );
            rule_ids.push_back(rule.id);
        }

        // At this point, count should be MAX_CONTEXT_RULES and we cannot add more
        // Remove the first rule
        remove_context_rule(&e, rule_ids.get(0).unwrap());

        // Now we should be able to add a new rule because count was decremented
        let new_contract_addr = Address::generate(&e);
        let new_context_type = ContextRuleType::CallContract(new_contract_addr);

        let new_rule = add_context_rule(
            &e,
            &new_context_type,
            &String::from_str(&e, "new_rule"),
            None,
            &signers,
            &policies_map,
        );

        // Verify the new rule was added successfully
        assert_eq!(new_rule.id, MAX_CONTEXT_RULES);

        // Remove multiple rules
        remove_context_rule(&e, rule_ids.get(1).unwrap());
        remove_context_rule(&e, rule_ids.get(2).unwrap());

        // Add two more rules to verify count tracking
        for i in 0..2 {
            let contract_addr = Address::generate(&e);
            let context_type = ContextRuleType::CallContract(contract_addr);

            add_context_rule(
                &e,
                &context_type,
                &String::from_str(&e, "additional_rule"),
                Some(200 + i),
                &signers,
                &policies_map,
            );
        }
    });
}
