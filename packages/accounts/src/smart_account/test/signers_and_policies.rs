#![cfg(test)]
extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events},
    Address, Bytes, Env, Map, String, Val, Vec,
};

use crate::{
    policies::Policy,
    smart_account::{
        storage::{
            add_context_rule, add_policy, add_signer, batch_add_signer, get_context_rule,
            remove_policy, remove_signer, validate_signers_and_policies, ContextRule,
            ContextRuleType, PolicyEntry, Signer, SignerEntry, SmartAccountStorageKey,
        },
        MAX_POLICIES, MAX_SIGNERS,
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
        _context: soroban_sdk::auth::Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(
        _e: &Env,
        _install_params: Self::AccountParams,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn uninstall(e: &Env, _rule: ContextRule, _smart_account: Address) {
        let block_uninstall = e.storage().persistent().get(&symbol_short!("veto")).unwrap_or(false);
        if block_uninstall {
            panic!("Veto Uninstall Policy")
        }
    }
}

fn create_test_signers(e: &Env) -> Vec<Signer> {
    let signer1 = Signer::Delegated(Address::generate(e));
    let signer2 = Signer::Delegated(Address::generate(e));
    Vec::from_array(e, [signer1, signer2])
}

// Helper to get signer ID from a rule by signer object
fn get_signer_id(e: &Env, rule_id: u32, signer: &Signer) -> u32 {
    let rule = get_context_rule(e, rule_id);
    let pos = rule.signers.iter().rposition(|s| s == *signer).expect("signer not found");
    let entry_key = SmartAccountStorageKey::ContextRuleData(rule_id);
    let entry: crate::smart_account::storage::ContextRuleEntry =
        e.storage().persistent().get(&entry_key).unwrap();
    entry.signer_ids.get_unchecked(pos as u32)
}

// Helper to get policy ID from a rule by policy address
fn get_policy_id(e: &Env, rule_id: u32, policy: &Address) -> u32 {
    let rule = get_context_rule(e, rule_id);
    let pos = rule.policies.iter().rposition(|p| p == *policy).expect("policy not found");
    let entry_key = SmartAccountStorageKey::ContextRuleData(rule_id);
    let entry: crate::smart_account::storage::ContextRuleEntry =
        e.storage().persistent().get(&entry_key).unwrap();
    entry.policy_ids.get_unchecked(pos as u32)
}

fn setup_test_rule(e: &Env, address: &Address) -> ContextRule {
    e.as_contract(address, || {
        let signers = create_test_signers(e);
        let contract_addr = Address::generate(e);

        add_context_rule(
            e,
            &ContextRuleType::CallContract(contract_addr),
            &String::from_str(e, "test_rule"),
            None,
            &signers,
            &Map::new(e),
        )
    })
}

// ################## SIGNER MANAGEMENT TESTS ##################

#[test]
fn add_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let new_signer = Signer::Delegated(Address::generate(&e));

        add_signer(&e, rule.id, &new_signer);

        let updated_rule = get_context_rule(&e, rule.id);
        // Events: 1 SignerAdded + 1 SignerRegistered = 2
        assert_eq!(e.events().all().events().len(), 2);
        assert_eq!(updated_rule.signers.len(), 3);
        assert!(updated_rule.signers.contains(&new_signer));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn add_signer_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let new_signer = Signer::Delegated(Address::generate(&e));
        add_signer(&e, 999, &new_signer); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3007)")]
fn add_signer_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let existing_signer = rule.signers.get(0).unwrap();
        add_signer(&e, rule.id, &existing_signer); // Duplicate signer
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3013)")]
fn add_signer_oversized_external_key_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let verifier = Address::generate(&e);
        let oversized_key = Bytes::from_slice(&e, &[0u8; 257]);
        let signer = Signer::External(verifier, oversized_key);
        add_signer(&e, rule.id, &signer);
    });
}

#[test]
fn remove_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let signer_to_remove = rule.signers.get(0).unwrap();
        let signer_id = get_signer_id(&e, rule.id, &signer_to_remove);

        remove_signer(&e, rule.id, signer_id);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.signers.len(), 1);
        // Events: 1 SignerRemoved + 1 SignerDeregistered = 2
        assert_eq!(e.events().all().events().len(), 2);
        assert!(!updated_rule.signers.contains(&signer_to_remove));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn remove_signer_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        remove_signer(&e, 999, 0); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3006)")]
fn remove_signer_not_found_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        remove_signer(&e, rule.id, 999); // Signer ID not in rule
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3004)")]
fn remove_signer_last_one_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        // Remove first signer - should succeed (still have one left)
        let signer1 = rule.signers.get(0).unwrap();
        let signer1_id = get_signer_id(&e, rule.id, &signer1);
        remove_signer(&e, rule.id, signer1_id);

        // Try to remove last signer - should fail with NoSignersAndPolicies
        let signer2 = rule.signers.get(1).unwrap();
        let signer2_id = get_signer_id(&e, rule.id, &signer2);
        remove_signer(&e, rule.id, signer2_id);
    });
}

#[test]
fn remove_signer_with_policy_present_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        // Add a policy first
        let install_param: Val = Val::from_void().into();
        add_policy(&e, rule.id, &policy_address, install_param);

        // Now we can remove all signers because we have a policy
        let signer1 = rule.signers.get(0).unwrap();
        let signer2 = rule.signers.get(1).unwrap();
        let signer1_id = get_signer_id(&e, rule.id, &signer1);
        let signer2_id = get_signer_id(&e, rule.id, &signer2);

        remove_signer(&e, rule.id, signer1_id);
        remove_signer(&e, rule.id, signer2_id);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.signers.len(), 0);
        assert_eq!(updated_rule.policies.len(), 1);
    });
}

#[test]
fn remove_signer_shared_across_rules_decrements_count() {
    // Register the same signer in two different rules. Removing it from the
    // first rule should decrement its reference count rather than delete it.
    let e = Env::default();
    let address = e.register(MockContract, ());
    let shared_signer = Signer::Delegated(Address::generate(&e));

    e.as_contract(&address, || {
        let contract_a = Address::generate(&e);
        let contract_b = Address::generate(&e);

        // rule_a has two signers so removing shared_signer leaves one remaining.
        let extra_signer_a = Signer::Delegated(Address::generate(&e));
        let rule_a = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_a),
            &String::from_str(&e, "rule_a"),
            None,
            &Vec::from_array(&e, [shared_signer.clone(), extra_signer_a]),
            &Map::new(&e),
        );

        // Different context type so the fingerprint differs; same signer re-used.
        let extra_signer = Signer::Delegated(Address::generate(&e));
        let _ = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_b),
            &String::from_str(&e, "rule_b"),
            None,
            &Vec::from_array(&e, [shared_signer.clone(), extra_signer]),
            &Map::new(&e),
        );

        // Remove the shared signer from rule_a (count 2 → 1, not deleted).
        let signer_id = get_signer_id(&e, rule_a.id, &shared_signer);
        remove_signer(&e, rule_a.id, signer_id);

        let count_key = SmartAccountStorageKey::SignerData(signer_id);
        let signer_data: SignerEntry = e.storage().persistent().get(&count_key).unwrap();
        assert_eq!(signer_data.count, 1);
    });
}

// ################## POLICY MANAGEMENT TESTS ##################

#[test]
fn add_policy_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();

        add_policy(&e, rule.id, &policy_address.clone(), install_param);

        let updated_rule = get_context_rule(&e, rule.id);
        // Events: 1 PolicyAdded + 1 PolicyRegistered = 2
        assert_eq!(e.events().all().events().len(), 2);
        assert_eq!(updated_rule.policies.len(), 1);
        assert!(updated_rule.policies.contains(&policy_address));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn add_policy_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();
        // Non-existent rule ID
        add_policy(&e, 999, &policy_address, install_param);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3009)")]
fn add_policy_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();

        // Add policy first time
        add_policy(&e, rule.id, &policy_address, install_param);

        // Try to add same policy again
        add_policy(&e, rule.id, &policy_address, install_param); // Duplicate policy
    });
}

#[test]
fn remove_policy_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();

        // First add a policy
        add_policy(&e, rule.id, &policy_address, install_param);

        // Then remove it
        let policy_id = get_policy_id(&e, rule.id, &policy_address);
        remove_policy(&e, rule.id, policy_id);

        let updated_rule = get_context_rule(&e, rule.id);
        // Events: 1 PolicyAdded + 1 PolicyRegistered + 1 PolicyRemoved + 1
        // PolicyDeregistered = 4
        assert_eq!(e.events().all().events().len(), 4);
        assert_eq!(updated_rule.policies.len(), 0);
        assert!(!updated_rule.policies.contains(&policy_address));
    });

    // case when `unistall` of the policy panics
    e.as_contract(&policy_address, || {
        e.storage().persistent().set(&symbol_short!("veto"), &true);
    });

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();

        add_policy(&e, rule.id, &policy_address, install_param);

        // Removal succeeds
        let policy_id = get_policy_id(&e, rule.id, &policy_address);
        remove_policy(&e, rule.id, policy_id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn remove_policy_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        remove_policy(&e, 999, 0); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3008)")]
fn remove_policy_not_found_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        remove_policy(&e, rule.id, 999); // Policy ID not in rule
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3004)")]
fn remove_policy_last_one_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    e.as_contract(&address, || {
        // Create a rule with only a policy, no signers
        let signers = Vec::new(&e);
        let mut policies_map = Map::new(&e);
        policies_map.set(policy_address.clone(), Val::from_void().into());

        let rule = add_context_rule(
            &e,
            &ContextRuleType::Default,
            &String::from_str(&e, "policy_only_rule"),
            None,
            &signers,
            &policies_map,
        );

        // Try to remove the only policy - should fail with NoSignersAndPolicies
        let policy_id = get_policy_id(&e, rule.id, &policy_address);
        remove_policy(&e, rule.id, policy_id);
    });
}

#[test]
fn remove_policy_with_signers_present_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        // Add a policy
        let install_param: Val = Val::from_void().into();
        add_policy(&e, rule.id, &policy_address, install_param);

        // Remove the policy - should succeed because we still have signers
        let policy_id = get_policy_id(&e, rule.id, &policy_address);
        remove_policy(&e, rule.id, policy_id);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.policies.len(), 0);
        assert_eq!(updated_rule.signers.len(), 2); // Still have signers
    });
}

#[test]
fn remove_policy_shared_across_rules_decrements_count() {
    // Register the same policy in two different rules. Removing it from the
    // first rule should decrement its reference count rather than delete it.
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());
    let install_param: Val = Val::from_void().into();

    e.as_contract(&address, || {
        let contract_a = Address::generate(&e);
        let contract_b = Address::generate(&e);

        let signers_a = Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]);
        let signers_b = Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]);

        let mut policies_map = Map::new(&e);
        policies_map.set(policy_address.clone(), install_param);

        let rule_a = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_a),
            &String::from_str(&e, "pol_rule_a"),
            None,
            &signers_a,
            &policies_map,
        );

        let _ = add_context_rule(
            &e,
            &ContextRuleType::CallContract(contract_b),
            &String::from_str(&e, "pol_rule_b"),
            None,
            &signers_b,
            &policies_map,
        );

        // Remove the policy from rule_a (count 2 → 1, not deleted).
        let policy_id = get_policy_id(&e, rule_a.id, &policy_address);
        remove_policy(&e, rule_a.id, policy_id);

        let count_key = SmartAccountStorageKey::PolicyData(policy_id);
        let policy_data: PolicyEntry = e.storage().persistent().get(&count_key).unwrap();
        assert_eq!(policy_data.count, 1);
    });
}

// ################## VALIDATION TESTS ##################

#[test]
fn validate_signers_and_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signer_ids = Vec::from_array(&e, [0u32]);
        let policy_ids = Vec::from_array(&e, [0u32]);

        // Should not panic
        validate_signers_and_policies(&e, &signer_ids, &policy_ids);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3004)")]
fn validate_signers_and_policies_no_signers_and_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signer_ids = Vec::new(&e);
        let policy_ids = Vec::new(&e);

        validate_signers_and_policies(&e, &signer_ids, &policy_ids);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3010)")]
fn validate_signers_and_policies_too_many_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let mut signer_ids = Vec::new(&e);
        for i in 0..=MAX_SIGNERS {
            signer_ids.push_back(i);
        }
        let policy_ids = Vec::new(&e);

        validate_signers_and_policies(&e, &signer_ids, &policy_ids);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3011)")]
fn validate_signers_and_policies_too_many_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signer_ids = Vec::new(&e);
        let mut policy_ids = Vec::new(&e);
        for i in 0..=MAX_POLICIES {
            policy_ids.push_back(i);
        }

        validate_signers_and_policies(&e, &signer_ids, &policy_ids);
    });
}

// ################## BATCH SIGNER MANAGEMENT TESTS ##################

#[test]
fn batch_add_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let new_signers = Vec::from_array(
            &e,
            [Signer::Delegated(Address::generate(&e)), Signer::Delegated(Address::generate(&e))],
        );

        batch_add_signer(&e, rule.id, &new_signers);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.signers.len(), 4); // 2 original + 2 new
        assert!(updated_rule.signers.contains(new_signers.get_unchecked(0)));
        assert!(updated_rule.signers.contains(new_signers.get_unchecked(1)));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3000)")]
fn batch_add_signer_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]);
        batch_add_signer(&e, 999, &signers);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3007)")]
fn batch_add_signer_duplicate_signer_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let existing_signer = rule.signers.get(0).unwrap();
        let signers = Vec::from_array(&e, [existing_signer]);
        batch_add_signer(&e, rule.id, &signers);
    });
}
