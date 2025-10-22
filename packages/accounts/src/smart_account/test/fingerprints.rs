extern crate std;

use soroban_sdk::{
    auth::Context, contract, contractimpl, map, testutils::Address as _, Address, Bytes, Env, Map,
    String, Val, Vec,
};

use crate::{
    policies::Policy,
    smart_account::storage::{
        add_context_rule, add_policy, add_signer, compute_fingerprint, remove_context_rule,
        remove_policy, remove_signer, validate_and_set_fingerprint, ContextRule, ContextRuleType,
        Signer, SmartAccountStorageKey,
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
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) -> bool {
        true
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

#[test]
fn compute_fingerprint_different_signers_different_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers1 = create_test_signers(&e);
    let addr3 = Address::generate(&e);
    let signers2 = Vec::from_array(&e, [Signer::Delegated(addr3)]);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers1, &policies);
        let fp2 = compute_fingerprint(&e, &context_type, &signers2, &policies);
        assert_ne!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_signers_different_order_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let signers1 =
        Vec::from_array(&e, [Signer::Delegated(addr1.clone()), Signer::Delegated(addr2.clone())]);
    let signers2 = Vec::from_array(&e, [Signer::Delegated(addr2), Signer::Delegated(addr1)]);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers1, &policies);
        let fp2 = compute_fingerprint(&e, &context_type, &signers2, &policies);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_different_policies_different_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = create_test_signers(&e);
    let policies1 = create_test_policies(&e);
    let policy3 = e.register(MockPolicyContract, ());
    let policies2 = Vec::from_array(&e, [policy3]);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers, &policies1);
        let fp2 = compute_fingerprint(&e, &context_type, &signers, &policies2);
        assert_ne!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_policies_different_order_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = Vec::new(&e);
    let policy1 = e.register(MockPolicyContract, ());
    let policy2 = e.register(MockPolicyContract, ());
    let policies_order1 = Vec::from_array(&e, [policy1.clone(), policy2.clone()]);
    let policies_order2 = Vec::from_array(&e, [policy2, policy1]);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers, &policies_order1);
        let fp2 = compute_fingerprint(&e, &context_type, &signers, &policies_order2);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_valid_until_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = create_test_signers(&e);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers, &policies);
        let fp2 = compute_fingerprint(&e, &context_type, &signers, &policies);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_mixed_signer_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let native_addr = Address::generate(&e);
    let verifier1 = Address::generate(&e);
    let verifier2 = Address::generate(&e);
    let key_data1 = Bytes::from_array(&e, &[1, 2, 3, 4]);
    let key_data2 = Bytes::from_array(&e, &[5, 6, 3, 4]);

    let native_signer = Signer::Delegated(native_addr);
    let external_signer1 = Signer::External(verifier1, key_data1);
    let external_signer2 = Signer::External(verifier2, key_data2);

    let signers1 = Vec::from_array(
        &e,
        [native_signer.clone(), external_signer1.clone(), external_signer2.clone()],
    );
    let signers2 = Vec::from_array(
        &e,
        [external_signer1.clone(), native_signer.clone(), external_signer2.clone()],
    );
    let signers3 = Vec::from_array(&e, [native_signer, external_signer2, external_signer1]);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        let fp1 = compute_fingerprint(&e, &context_type, &signers1, &policies);
        let fp2 = compute_fingerprint(&e, &context_type, &signers2, &policies);
        let fp3 = compute_fingerprint(&e, &context_type, &signers3, &policies);
        assert_eq!(fp1, fp2);
        assert_eq!(fp3, fp2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3007)")]
fn compute_fingerprint_duplicate_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let addr1 = Address::generate(&e);
    let duplicate_signers =
        Vec::from_array(&e, [Signer::Delegated(addr1.clone()), Signer::Delegated(addr1)]);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        let _ = compute_fingerprint(&e, &context_type, &duplicate_signers, &policies);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3009)")]
fn compute_fingerprint_duplicate_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = Vec::new(&e);
    let policy1 = e.register(MockPolicyContract, ());
    let duplicate_policies = Vec::from_array(&e, [policy1.clone(), policy1]);

    e.as_contract(&address, || {
        let _ = compute_fingerprint(&e, &context_type, &signers, &duplicate_policies);
    });
}

#[test]
fn validate_and_set_fingerprint_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = create_test_signers(&e);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        validate_and_set_fingerprint(&e, &context_type, &signers, &policies);
        let fp = compute_fingerprint(&e, &context_type, &signers, &policies);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp)))
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3001)")]
fn validate_and_set_fingerprint_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = create_test_signers(&e);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        // First call should succeed
        validate_and_set_fingerprint(&e, &context_type, &signers, &policies);

        // Second call with same parameters should fail
        validate_and_set_fingerprint(&e, &context_type, &signers, &policies);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3001)")]
fn validate_and_set_fingerprint_same_valid_until_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr);
    let signers = create_test_signers(&e);
    let policies = Vec::new(&e);

    e.as_contract(&address, || {
        // Same fingerprint should fail since valid_until is not part of fingerprint
        validate_and_set_fingerprint(&e, &context_type, &signers, &policies);
        validate_and_set_fingerprint(&e, &context_type, &signers, &policies);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3001)")]
fn add_context_rule_duplicate_fingerprint_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr.clone());
    let signers = create_test_signers(&e);

    e.as_contract(&address, || {
        // Add first rule
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        // Try to add second rule with same signers, policies, and valid_until
        // Should fail even with different name or context type
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule2"),
            None,
            &signers,
            &Map::new(&e),
        );
    });
}

#[test]
fn add_context_rule_different_context_type_same_signers_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr1 = Address::generate(&e);
    let contract_addr2 = Address::generate(&e);
    let signers = create_test_signers(&e);

    e.as_contract(&address, || {
        // Add first rule for contract_addr1
        add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr1),
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        // Should succeed - different context types have different fingerprints
        // Fingerprint includes context_type, signers, policies, and valid_until
        add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_addr2),
            &String::from_str(&e, "rule2"),
            None,
            &signers,
            &Map::new(&e),
        );
    });
}

#[test]
fn remove_context_rule_removes_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr.clone());
    let signers = create_test_signers(&e);

    e.as_contract(&address, || {
        // Add rule
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        // Remove rule
        remove_context_rule(&e, rule.id);
        let fp = compute_fingerprint(&e, &context_type, &signers, &Vec::new(&e));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp)))
    });
}

#[test]
fn add_signer_updates_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        let addr1 = Address::generate(&e);
        let mut signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);

        // Add rule with one signer
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );
        let fp1 = compute_fingerprint(&e, &context_type, &signers, &Vec::new(&e));
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        // Add another signer
        let signer2 = Signer::Delegated(Address::generate(&e));
        add_signer(&e, rule.id, &signer2);

        signers.push_back(signer2);
        let fp2 = compute_fingerprint(&e, &context_type, &signers, &Vec::new(&e));
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp2)));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1)));
    });
}

#[test]
fn remove_signer_updates_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        let mut signers = create_test_signers(&e);

        // Add rule with two signers and a policy to satisfy minimum requirements
        let policy = e.register(MockPolicyContract, ());
        let policies_map = map![&e, (policy.clone(), Val::from_void().into())];

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &policies_map,
        );
        let fp1 = compute_fingerprint(&e, &context_type, &signers, &policies_map.keys());
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        // Remove one signer
        let signer_to_remove = signers.pop_back().unwrap();
        remove_signer(&e, rule.id, &signer_to_remove);

        let fp2 = compute_fingerprint(&e, &context_type, &signers, &policies_map.keys());
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp2)));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1)));
    });
}

#[test]
fn add_policy_updates_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        let signers = create_test_signers(&e);

        // Add rule with no policies
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        let fp1 = compute_fingerprint(&e, &context_type, &signers, &Vec::new(&e));
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        // Add a policy
        let policy = e.register(MockPolicyContract, ());
        add_policy(&e, rule.id, &policy, Val::from_void().into());

        let fp2 = compute_fingerprint(&e, &context_type, &signers, &Vec::from_slice(&e, &[policy]));
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp2)));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1)));
    });
}

#[test]
fn remove_policy_updates_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        let signers = create_test_signers(&e);
        let policy = e.register(MockPolicyContract, ());
        let policies_map = map![&e, (policy.clone(), Val::from_void().into())];

        // Add rule with one policy
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &policies_map,
        );

        let fp1 = compute_fingerprint(&e, &context_type, &signers, &policies_map.keys());
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        // Remove the policy
        remove_policy(&e, rule.id, &policy);
        let fp2 = compute_fingerprint(&e, &context_type, &signers, &Vec::new(&e));
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp2)));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1)));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3001)")]
fn fingerprint_prevents_duplicate_rules_across_modifications() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());
        let addr1 = Address::generate(&e);
        let addr2 = Address::generate(&e);
        let addr3 = Address::generate(&e);

        let signers_1_2 = Vec::from_array(
            &e,
            [Signer::Delegated(addr1.clone()), Signer::Delegated(addr2.clone())],
        );
        let signers_1_2_3 = Vec::from_array(
            &e,
            [Signer::Delegated(addr1), Signer::Delegated(addr2), Signer::Delegated(addr3.clone())],
        );

        // Add rule with signers [addr1, addr2]
        let _rule1 = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers_1_2,
            &Map::new(&e),
        );

        // Add rule with signers [addr1, addr2, addr3] and a policy
        let policy = e.register(MockPolicyContract, ());

        let rule2 = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule2"),
            None,
            &signers_1_2_3,
            &map![&e, (policy.clone(), Val::from_void().into())],
        );

        // Remove addr3 and policy from rule2
        remove_signer(&e, rule2.id, &Signer::Delegated(addr3));
        remove_policy(&e, rule2.id, &policy);

        // Now rule2 has [addr1, addr2] with no policies, which conflicts with rule1
        // Trying to add another rule with [addr1, addr2] should fail
        add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule3"),
            None,
            &signers_1_2,
            &Map::new(&e),
        );
    });
}
