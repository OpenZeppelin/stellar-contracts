extern crate std;

use soroban_sdk::{
    auth::Context, contract, contractimpl, map, testutils::Address as _, Address, Env, Map, String,
    Val, Vec,
};

use crate::{
    policies::Policy,
    smart_account::storage::{
        add_context_rule, add_policy, add_signer, compute_fingerprint, remove_context_rule,
        remove_policy, remove_signer, set_fingerprint, ContextRule, ContextRuleType, Signer,
        SmartAccountStorageKey,
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

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

fn create_test_signers(e: &Env) -> Vec<Signer> {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);

    Vec::from_array(e, [Signer::Delegated(addr1), Signer::Delegated(addr2)])
}

fn entry_signer_ids(e: &Env, rule_id: u32) -> Vec<u32> {
    let entry: crate::smart_account::storage::ContextRuleEntry =
        e.storage().persistent().get(&SmartAccountStorageKey::ContextRuleData(rule_id)).unwrap();
    entry.signer_ids
}

fn entry_policy_ids(e: &Env, rule_id: u32) -> Vec<u32> {
    let entry: crate::smart_account::storage::ContextRuleEntry =
        e.storage().persistent().get(&SmartAccountStorageKey::ContextRuleData(rule_id)).unwrap();
    entry.policy_ids
}

#[test]
fn compute_fingerprint_different_signers_different_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let ids1 = Vec::from_array(&e, [0u32, 1u32]);
        let ids2 = Vec::from_array(&e, [2u32]);
        let policy_ids: Vec<u32> = Vec::new(&e);
        let fp1 = compute_fingerprint(&e, &context_type, &ids1, &policy_ids);
        let fp2 = compute_fingerprint(&e, &context_type, &ids2, &policy_ids);
        assert_ne!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_signers_different_order_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let ids = Vec::from_array(&e, [0u32, 1u32]);
        let ids_reversed = Vec::from_array(&e, [1u32, 0u32]);
        let policy_ids: Vec<u32> = Vec::new(&e);
        let fp1 = compute_fingerprint(&e, &context_type, &ids, &policy_ids);
        let fp2 = compute_fingerprint(&e, &context_type, &ids_reversed, &policy_ids);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_different_policies_different_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let signer_ids = Vec::from_array(&e, [0u32, 1u32]);
        let policy_ids1 = Vec::from_array(&e, [0u32, 1u32]);
        let policy_ids2 = Vec::from_array(&e, [2u32]);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids1);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids2);
        assert_ne!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_policies_different_order_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let signer_ids = Vec::from_array(&e, [0u32, 1u32]);
        let policy_ids = Vec::from_array(&e, [0u32, 1u32]);
        let policy_ids_reversed = Vec::from_array(&e, [1u32, 0u32]);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids_reversed);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_same_inputs_same_fingerprint() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let signer_ids = Vec::from_array(&e, [0u32, 1u32]);
        let policy_ids: Vec<u32> = Vec::new(&e);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids);
        assert_eq!(fp1, fp2);
    });
}

#[test]
fn compute_fingerprint_order_independent() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let policy_ids: Vec<u32> = Vec::new(&e);
        let ids = Vec::from_array(&e, [0u32, 1u32, 2u32]);
        let ids2 = Vec::from_array(&e, [1u32, 0u32, 2u32]);
        let ids3 = Vec::from_array(&e, [0u32, 2u32, 1u32]);
        let fp1 = compute_fingerprint(&e, &context_type, &ids, &policy_ids);
        let fp2 = compute_fingerprint(&e, &context_type, &ids2, &policy_ids);
        let fp3 = compute_fingerprint(&e, &context_type, &ids3, &policy_ids);
        assert_eq!(fp1, fp2);
        assert_eq!(fp1, fp3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3007)")]
fn compute_fingerprint_duplicate_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let duplicate_ids = Vec::from_array(&e, [0u32, 0u32]);
        let policy_ids: Vec<u32> = Vec::new(&e);
        let _ = compute_fingerprint(&e, &context_type, &duplicate_ids, &policy_ids);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3009)")]
fn compute_fingerprint_duplicate_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let context_type = ContextRuleType::CallContract(Address::generate(&e));

    e.as_contract(&address, || {
        let signer_ids: Vec<u32> = Vec::new(&e);
        let duplicate_policy_ids = Vec::from_array(&e, [0u32, 0u32]);
        let _ = compute_fingerprint(&e, &context_type, &signer_ids, &duplicate_policy_ids);
    });
}

#[test]
fn set_fingerprint_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr.clone());
    let signers = create_test_signers(&e);

    e.as_contract(&address, || {
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "r1"),
            None,
            &signers,
            &Map::new(&e),
        );
        let signer_ids = entry_signer_ids(&e, rule.id);
        let policy_ids: Vec<u32> = Vec::new(&e);

        // Remove the fingerprint that add_context_rule set so we can re-test
        // set_fingerprint in isolation on a different context type
        let context_type2 = ContextRuleType::CallContract(Address::generate(&e));
        set_fingerprint(&e, &context_type2, &signer_ids, &policy_ids);
        let fp = compute_fingerprint(&e, &context_type2, &signer_ids, &policy_ids);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp)));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3001)")]
fn set_fingerprint_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let contract_addr = Address::generate(&e);
    let context_type = ContextRuleType::CallContract(contract_addr.clone());
    let signers = create_test_signers(&e);

    e.as_contract(&address, || {
        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "r1"),
            None,
            &signers,
            &Map::new(&e),
        );
        let signer_ids = entry_signer_ids(&e, rule.id);
        let policy_ids: Vec<u32> = Vec::new(&e);

        // Use a fresh context type so we start clean
        let context_type2 = ContextRuleType::CallContract(Address::generate(&e));
        set_fingerprint(&e, &context_type2, &signer_ids, &policy_ids);
        // Second call with same parameters should fail
        set_fingerprint(&e, &context_type2, &signer_ids, &policy_ids);
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

        // Capture IDs before removal to verify the fingerprint is gone afterwards
        let signer_ids = entry_signer_ids(&e, rule.id);
        let policy_ids = entry_policy_ids(&e, rule.id);
        let fp = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp.clone())));

        remove_context_rule(&e, rule.id);
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp)));
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
        let signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        let signer_ids_before = entry_signer_ids(&e, rule.id);
        let policy_ids: Vec<u32> = Vec::new(&e);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids_before, &policy_ids);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        let signer2 = Signer::Delegated(Address::generate(&e));
        add_signer(&e, rule.id, &signer2);

        let signer_ids_after = entry_signer_ids(&e, rule.id);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids_after, &policy_ids);
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
        let signers = create_test_signers(&e);
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

        let signer_ids_before = entry_signer_ids(&e, rule.id);
        let policy_ids = entry_policy_ids(&e, rule.id);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids_before, &policy_ids);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        // Remove the last signer ID
        let signer_id_to_remove = signer_ids_before.get_unchecked(signer_ids_before.len() - 1);
        remove_signer(&e, rule.id, signer_id_to_remove);

        let signer_ids_after = entry_signer_ids(&e, rule.id);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids_after, &policy_ids);
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

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &Map::new(&e),
        );

        let signer_ids = entry_signer_ids(&e, rule.id);
        let policy_ids_before = entry_policy_ids(&e, rule.id);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids_before);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        let policy = e.register(MockPolicyContract, ());
        add_policy(&e, rule.id, &policy, Val::from_void().into());

        let policy_ids_after = entry_policy_ids(&e, rule.id);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids_after);
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

        let rule = add_context_rule(
            &e,
            &context_type,
            &String::from_str(&e, "rule1"),
            None,
            &signers,
            &policies_map,
        );

        let signer_ids = entry_signer_ids(&e, rule.id);
        let policy_ids_before = entry_policy_ids(&e, rule.id);
        let fp1 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids_before);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1.clone())));

        let policy_id = policy_ids_before.get_unchecked(0);
        remove_policy(&e, rule.id, policy_id);

        let policy_ids_after = entry_policy_ids(&e, rule.id);
        let fp2 = compute_fingerprint(&e, &context_type, &signer_ids, &policy_ids_after);
        assert!(e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp2)));
        assert!(!e.storage().persistent().has(&SmartAccountStorageKey::Fingerprint(fp1)));
    });
}
