#![cfg(test)]
extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    Address, Env, Map, String, Val, Vec,
};

use super::super::storage::{
    add_context_rule, add_policy, add_signer, get_context_rule, remove_policy, remove_signer,
    ContextRule, ContextRuleType, Signer,
};
use crate::policies::Policy;

#[contract]
struct MockContract;

#[contractimpl]
impl MockContract {
    pub fn test() {}
}

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type InstallParams = Val;

    fn can_enforce(
        _e: &Env,
        _context: soroban_sdk::auth::Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) -> bool {
        true
    }

    fn enforce(
        _e: &Env,
        _context: soroban_sdk::auth::Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) {
    }

    fn install(_e: &Env, _install_params: Self::InstallParams, _smart_account: Address) {}

    fn uninstall(_e: &Env, _smart_account: Address) {}
}

fn create_test_signers(e: &Env) -> Vec<Signer> {
    let signer1 = Signer::Native(Address::generate(e));
    let signer2 = Signer::Native(Address::generate(e));
    Vec::from_array(e, [signer1, signer2])
}

fn setup_test_rule(e: &Env, address: &Address) -> ContextRule {
    e.as_contract(address, || {
        let signers = create_test_signers(e);
        let contract_addr = Address::generate(e);

        add_context_rule(
            e,
            &ContextRuleType::CallContract(contract_addr),
            String::from_str(e, "test_rule"),
            None,
            signers,
            Map::new(e),
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
        let new_signer = Signer::Native(Address::generate(&e));

        add_signer(&e, rule.id, new_signer.clone());

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(e.events().all().len(), 1);
        assert_eq!(updated_rule.signers.len(), 3);
        assert!(updated_rule.signers.contains(&new_signer));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn add_signer_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let new_signer = Signer::Native(Address::generate(&e));
        add_signer(&e, 999, new_signer); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2007)")]
fn add_signer_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let existing_signer = rule.signers.get(0).unwrap();
        add_signer(&e, rule.id, existing_signer); // Duplicate signer
    });
}

#[test]
fn remove_signer_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let signer_to_remove = rule.signers.get(0).unwrap();

        remove_signer(&e, rule.id, signer_to_remove.clone());

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.signers.len(), 1);
        assert_eq!(e.events().all().len(), 1);
        assert!(!updated_rule.signers.contains(&signer_to_remove));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn remove_signer_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signer = Signer::Native(Address::generate(&e));
        remove_signer(&e, 999, signer); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2006)")]
fn remove_signer_not_found_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let nonexistent_signer = Signer::Native(Address::generate(&e));
        remove_signer(&e, rule.id, nonexistent_signer); // Signer not in rule
    });
}

#[test]
fn remove_signer_last_one_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        // Remove all signers one by one
        let signer1 = rule.signers.get(0).unwrap();
        let signer2 = rule.signers.get(1).unwrap();

        remove_signer(&e, rule.id, signer1);
        remove_signer(&e, rule.id, signer2);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(updated_rule.signers.len(), 0);
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

        add_policy(&e, rule.id, policy_address.clone(), install_param);

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(e.events().all().len(), 1);
        assert_eq!(updated_rule.policies.len(), 1);
        assert!(updated_rule.policies.contains(&policy_address));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn add_policy_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();
        add_policy(&e, 999, policy_address, install_param); // Non-existent rule
                                                            // ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2009)")]
fn add_policy_duplicate_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let install_param: Val = Val::from_void().into();

        // Add policy first time
        add_policy(&e, rule.id, policy_address.clone(), install_param);

        // Try to add same policy again
        add_policy(&e, rule.id, policy_address, install_param); // Duplicate policy
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
        add_policy(&e, rule.id, policy_address.clone(), install_param);

        // Then remove it
        remove_policy(&e, rule.id, policy_address.clone());

        let updated_rule = get_context_rule(&e, rule.id);
        assert_eq!(e.events().all().len(), 2);
        assert_eq!(updated_rule.policies.len(), 0);
        assert!(!updated_rule.policies.contains(&policy_address));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn remove_policy_nonexistent_rule_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    e.as_contract(&address, || {
        remove_policy(&e, 999, policy_address); // Non-existent rule ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2008)")]
fn remove_policy_not_found_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let policy_address = e.register(MockPolicyContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        remove_policy(&e, rule.id, policy_address); // Policy not in rule
    });
}
