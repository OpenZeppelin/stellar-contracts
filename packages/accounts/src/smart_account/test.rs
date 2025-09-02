#![cfg(test)]
extern crate std;

use soroban_sdk::{
    auth::{
        Context, ContractContext, CreateContractHostFnContext,
        CreateContractWithConstructorHostFnContext,
    },
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Bytes, BytesN, Env, Map, String, Symbol, Val, Vec,
};

use super::{
    enforce_policy,
    storage::{
        add_context_rule, authenticate, can_enforce_all_policies, get_authenticated_signers,
        get_context_rule, get_valid_context_rules, modify_context_rule, remove_context_rule,
        ContextRule, ContextRuleType, ContextRuleVal, Signatures, Signer,
    },
};
use crate::smart_account::{get_context_rules, get_validated_context, storage::do_check_auth};

#[contract]
struct MockContract;

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl MockPolicyContract {
    pub fn can_enforce(
        e: &Env,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) -> bool {
        e.storage().persistent().get(&symbol_short!("enforce")).unwrap_or(true)
    }

    pub fn enforce(
        _e: &Env,
        _context: Context,
        _context_rule_signers: Vec<Signer>,
        _authenticated_signers: Vec<Signer>,
        _smart_account: Address,
    ) {
    }
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(e: &Env, _hash: Bytes, _key_data: Bytes, _sig_data: Bytes) -> bool {
        e.storage().persistent().get(&symbol_short!("verify")).unwrap_or(true)
    }
}

fn create_test_signers(e: &Env) -> Vec<Signer> {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);

    Vec::from_array(e, [Signer::Native(addr1), Signer::Native(addr2)])
}

fn create_test_policies(e: &Env) -> Vec<Address> {
    let policy1 = e.register(MockPolicyContract, ());
    let policy2 = e.register(MockPolicyContract, ());

    Vec::from_array(e, [policy1, policy2])
}

fn setup_test_rule(e: &Env, address: &Address) -> ContextRule {
    e.as_contract(address, || {
        let signers = create_test_signers(e);
        let policies = create_test_policies(e);
        let contract_addr = Address::generate(e);

        let rule_val = ContextRuleVal {
            name: String::from_str(e, "test_rule"),
            signers,
            policies,
            valid_until: None,
        };

        add_context_rule(e, &ContextRuleType::CallContract(contract_addr), &rule_val)
    })
}

fn create_rule_val_with_signers(e: &Env, name: &str) -> ContextRuleVal {
    ContextRuleVal {
        name: String::from_str(e, name),
        signers: create_test_signers(e),
        policies: Vec::new(e),
        valid_until: None,
    }
}

fn create_rule_val_with_policies(e: &Env, name: &str) -> ContextRuleVal {
    ContextRuleVal {
        name: String::from_str(e, name),
        signers: Vec::new(e),
        policies: create_test_policies(e),
        valid_until: None,
    }
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
        add_context_rule(&e, &context_type, &create_rule_val_with_policies(&e, "policy_rule"));

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);
        let signatures = create_signatures(&e, &Vec::new(&e));
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let result =
            do_check_auth(e.clone(), e.crypto().sha256(&payload), signatures, auth_contexts);

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
        let rule1 =
            add_context_rule(&e, &context_type1, &create_rule_val_with_signers(&e, "rule1"));
        let rule2 =
            add_context_rule(&e, &context_type2, &create_rule_val_with_signers(&e, "rule2"));

        let context1 = get_context(contract_addr1, symbol_short!("test1"), vec![&e]);
        let context2 = get_context(contract_addr2, symbol_short!("test2"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context1, context2]);

        // Create signatures with all required signers
        let mut all_signers = rule1.signers.clone();
        all_signers.append(&rule2.signers);
        let signatures = create_signatures(&e, &all_signers);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let result =
            do_check_auth(e.clone(), e.crypto().sha256(&payload), signatures, auth_contexts);

        assert!(result.is_ok());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn do_check_auth_authentication_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier_addr = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Create rule with delegated signer
        let key_data = Bytes::from_array(&e, &[1, 2, 3, 4]);
        let delegated_signer = Signer::Delegated(verifier_addr.clone(), key_data.clone());
        let rule_val = ContextRuleVal {
            name: String::from_str(&e, "delegated_rule"),
            signers: Vec::from_array(&e, [delegated_signer.clone()]),
            policies: Vec::new(&e),
            valid_until: None,
        };
        add_context_rule(&e, &context_type, &rule_val);

        // Set verifier to return false
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &false);
        });

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);

        let mut signature_map = Map::new(&e);
        signature_map.set(delegated_signer, Bytes::from_array(&e, &[5, 6, 7, 8]));
        let signatures = Signatures(signature_map);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let _ = do_check_auth(e.clone(), e.crypto().sha256(&payload), signatures, auth_contexts);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn do_check_auth_context_validation_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.mock_all_auths();

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule requiring 2 signers
        let rule =
            add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "strict_rule"));

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let auth_contexts = Vec::from_array(&e, [context]);

        // Provide insufficient signers
        let insufficient_signers = rule.signers.slice(..1);
        let signatures = create_signatures(&e, &insufficient_signers);
        let payload = Bytes::from_array(&e, &[1u8; 32]);

        let _ = do_check_auth(e.clone(), e.crypto().sha256(&payload), signatures, auth_contexts);
    });
}
#[test]
fn add_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies = create_test_policies(&e);
        let contract_addr = Address::generate(&e);
        let future_sequence = e.ledger().sequence() + 1000;

        let rule_val = ContextRuleVal {
            name: String::from_str(&e, "test_rule"),
            signers,
            policies,
            valid_until: Some(future_sequence),
        };

        let rule = add_context_rule(&e, &ContextRuleType::CallContract(contract_addr), &rule_val);

        assert_eq!(rule.id, 0);
        assert_eq!(rule.name, String::from_str(&e, "test_rule"));
        assert_eq!(rule.signers.len(), 2);
        assert_eq!(rule.policies.len(), 2);
        assert_eq!(rule.valid_until, Some(future_sequence));
    });
}

#[test]
fn add_context_rule_multiple_rules_increment_id() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies = Vec::new(&e);
        let contract_addr = Address::generate(&e);

        let rule_val1 = ContextRuleVal {
            name: String::from_str(&e, "rule_1"),
            signers: signers.clone(),
            policies: policies.clone(),
            valid_until: None,
        };

        let rule_val2 = ContextRuleVal {
            name: String::from_str(&e, "rule_2"),
            signers,
            policies,
            valid_until: None,
        };

        let rule1 =
            add_context_rule(&e, &ContextRuleType::CallContract(contract_addr.clone()), &rule_val1);

        let rule2 = add_context_rule(&e, &ContextRuleType::CallContract(contract_addr), &rule_val2);

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
        let policies = Vec::new(&e);
        let contract_addr = Address::generate(&e);
        let wasm_hash = BytesN::from_array(&e, &[1u8; 32]);

        let rule_val1 = ContextRuleVal {
            name: String::from_str(&e, "call_rule"),
            signers: signers.clone(),
            policies: policies.clone(),
            valid_until: None,
        };

        let rule_val2 = ContextRuleVal {
            name: String::from_str(&e, "create_rule"),
            signers,
            policies,
            valid_until: None,
        };

        let call_rule =
            add_context_rule(&e, &ContextRuleType::CallContract(contract_addr), &rule_val1);

        let create_rule =
            add_context_rule(&e, &ContextRuleType::CreateContract(wasm_hash), &rule_val2);

        assert_eq!(call_rule.id, 0);
        assert_eq!(create_rule.id, 1);
        assert!(matches!(call_rule.context_type, ContextRuleType::CallContract(_)));
        assert!(matches!(create_rule.context_type, ContextRuleType::CreateContract(_)));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn add_context_rule_no_signers_and_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = Vec::new(&e);
        let policies = Vec::new(&e);
        let contract_addr = Address::generate(&e);

        let rule_val = ContextRuleVal {
            name: String::from_str(&e, "empty_rule"),
            signers,
            policies,
            valid_until: None,
        };

        add_context_rule(&e, &ContextRuleType::CallContract(contract_addr), &rule_val);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn add_context_rule_past_valid_until_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let signers = create_test_signers(&e);
        let policies = Vec::new(&e);
        let contract_addr = Address::generate(&e);
        //let current = e.ledger().sequence();
        e.ledger().set_sequence_number(100);

        let rule_val = ContextRuleVal {
            name: String::from_str(&e, "expired_rule"),
            signers,
            policies,
            valid_until: Some(99),
        };

        add_context_rule(&e, &ContextRuleType::CallContract(contract_addr), &rule_val);
    });
}

#[test]
fn modify_context_rule_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let future_sequence = e.ledger().sequence() + 500;
        let mut rule_val = create_rule_val_with_policies(&e, "modified_rule");
        rule_val.valid_until = Some(future_sequence);

        let modified_rule = modify_context_rule(&e, rule.id, &rule_val);

        assert_eq!(modified_rule.id, rule.id);
        assert_eq!(modified_rule.name, String::from_str(&e, "modified_rule"));
        assert_eq!(modified_rule.signers.len(), 0);
        assert_eq!(modified_rule.policies.len(), 2);
        assert_eq!(modified_rule.valid_until, Some(future_sequence));

        // Verify it was actually stored
        let rule = get_context_rule(&e, rule.id);
        assert_eq!(rule.name, String::from_str(&e, "modified_rule"));

        // Modify again new valid_until None
        rule_val.valid_until = None;
        let modified_rule = modify_context_rule(&e, rule.id, &rule_val);

        assert_eq!(modified_rule.id, rule.id);
        assert_eq!(modified_rule.valid_until, None);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn modify_context_rule_nonexistent_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let new_rule_val = create_rule_val_with_signers(&e, "nonexistent");

        modify_context_rule(&e, 999, &new_rule_val); // Non-existent ID
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn modify_context_rule_no_signers_and_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        let empty_rule_val = ContextRuleVal {
            name: String::from_str(&e, "empty_modified"),
            signers: Vec::new(&e),
            policies: Vec::new(&e),
            valid_until: None,
        };

        modify_context_rule(&e, rule.id, &empty_rule_val);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn modify_context_rule_past_valid_until_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let rule = setup_test_rule(&e, &address);

    e.as_contract(&address, || {
        e.ledger().set_sequence_number(100);

        let mut past_rule_val = create_rule_val_with_signers(&e, "past_modified");
        past_rule_val.valid_until = Some(99);

        modify_context_rule(&e, rule.id, &past_rule_val);
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
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn remove_context_rule_nonexistent_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        remove_context_rule(&e, 999); // Non-existent ID
    });
}

#[test]
fn enforce_policy_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let rule = setup_test_rule(&e, &address);
        let mut authenticated = rule.signers.clone();
        authenticated.pop_back();

        enforce_policy(
            &e,
            &rule.policies.get(0).unwrap(),
            &get_context(Address::generate(&e), symbol_short!("name"), vec![&e]),
            &rule.signers,
            &authenticated,
        );
    });
}

#[test]
fn can_enforce_all_policies_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let policies = create_test_policies(&e);
        let rule_signers = create_test_signers(&e);
        let matched_signers = rule_signers.clone();
        let context = get_context(Address::generate(&e), symbol_short!("test"), vec![&e]);

        let result =
            can_enforce_all_policies(&e, &context, &policies, &rule_signers, &matched_signers);
        assert!(result);
    });
}

#[test]
fn can_enforce_all_policies_one_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let policies = create_test_policies(&e);
        let rule_signers = create_test_signers(&e);
        let matched_signers = rule_signers.clone();
        let context = get_context(Address::generate(&e), symbol_short!("test"), vec![&e]);

        // Set first policy to return true, second to return false
        e.as_contract(&policies.get(0).unwrap(), || {
            e.storage().persistent().set(&symbol_short!("enforce"), &true);
        });
        e.as_contract(&policies.get(1).unwrap(), || {
            e.storage().persistent().set(&symbol_short!("enforce"), &false);
        });

        let result =
            can_enforce_all_policies(&e, &context, &policies, &rule_signers, &matched_signers);
        assert!(!result); // Should fail because one policy returns false
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn authenticate_delegated_signer_verification_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let verifier_addr = e.register(MockVerifierContract, ());

    e.as_contract(&address, || {
        let key_data = Bytes::from_array(&e, &[1, 2, 3, 4]);
        let sig_data = Bytes::from_array(&e, &[5, 6, 7, 8]);
        let signer = Signer::Delegated(verifier_addr.clone(), key_data.clone());

        // Set verifier to return false
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &false);
        });

        let mut signature_map = Map::new(&e);
        signature_map.set(signer, sig_data);
        let signatures = Signatures(signature_map);

        let payload = Bytes::from_array(&e, &[1u8; 32]);

        authenticate(&e, &e.crypto().sha256(&payload), &signatures);
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
        let key_data = Bytes::from_array(&e, &[1, 2, 3, 4]);

        let native_signer = Signer::Native(native_addr);
        let delegated_signer = Signer::Delegated(verifier_addr.clone(), key_data);

        // Set verifier to return true
        e.as_contract(&verifier_addr, || {
            e.storage().persistent().set(&symbol_short!("verify"), &true);
        });

        let mut signature_map = Map::new(&e);
        signature_map.set(native_signer, Bytes::new(&e));
        signature_map.set(delegated_signer, Bytes::from_array(&e, &[5, 6, 7, 8]));
        let signatures = Signatures(signature_map);

        let payload = Bytes::from_array(&e, &[1u8; 32]);

        authenticate(&e, &e.crypto().sha256(&payload), &signatures);
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

        let rule_signers =
            Vec::from_array(&e, [Signer::Native(addr1.clone()), Signer::Native(addr2.clone())]);

        let all_signers = Vec::from_array(
            &e,
            [
                Signer::Native(addr1.clone()),
                Signer::Native(addr3), // addr2 is missing, addr3 is extra
            ],
        );

        let authenticated = get_authenticated_signers(&e, &rule_signers, &all_signers);

        assert_eq!(authenticated.len(), 1); // Only addr1 matches
        assert_eq!(authenticated.get(0).unwrap(), Signer::Native(addr1));
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

        let rule_signers = Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);

        let all_signers = Vec::from_array(&e, [Signer::Native(addr3), Signer::Native(addr4)]);

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
        add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "rule1"));
        add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "rule2"));

        let valid_rules = get_valid_context_rules(&e, context_type);

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

        add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "matched"));

        // Add default rules
        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &create_rule_val_with_signers(&e, "default"),
        );

        let valid_rules = get_valid_context_rules(&e, context_type);

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

        // Add only default rules
        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &create_rule_val_with_signers(&e, "default1"),
        );
        add_context_rule(
            &e,
            &ContextRuleType::Default,
            &create_rule_val_with_signers(&e, "default2"),
        );

        let valid_rules = get_valid_context_rules(&e, context_type);

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
        let mut valid_rule_val = create_rule_val_with_signers(&e, "valid");
        valid_rule_val.valid_until = Some(200);
        add_context_rule(&e, &context_type, &valid_rule_val);

        // Add expired rule
        let mut expired_rule_val = create_rule_val_with_signers(&e, "expired");
        expired_rule_val.valid_until = Some(150);
        add_context_rule(&e, &context_type, &expired_rule_val);

        // Forward ledger sequence
        e.ledger().set_sequence_number(160);

        // Add rule with no expiration
        add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "no_expiry"));

        let valid_rules = get_valid_context_rules(&e, context_type);

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

        let valid_rules = get_valid_context_rules(&e, context_type);

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
        let rule =
            add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "test_rule"));

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
        let rule =
            add_context_rule(&e, &context_type, &create_rule_val_with_policies(&e, "policy_rule"));

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
        let rule =
            add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "create_rule"));

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
#[should_panic(expected = "Error(Contract, #2)")]
fn get_validated_context_call_contract_with_policies_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule with policies
        let context_val = create_rule_val_with_policies(&e, "policy_rule");
        let failing_policy = context_val.policies.get(1).unwrap();
        e.as_contract(&failing_policy, || {
            e.storage().persistent().set(&symbol_short!("enforce"), &false);
        });
        add_context_rule(&e, &context_type, &context_val);

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        get_validated_context(&e, &context, &Vec::new(&e));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn get_validated_context_insufficient_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let contract_addr = Address::generate(&e);
        let context_type = ContextRuleType::CallContract(contract_addr.clone());

        // Add rule requiring 2 signers
        let rule =
            add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "strict_rule"));

        let context = get_context(contract_addr, symbol_short!("test"), vec![&e]);
        let insufficient_signers = rule.signers.slice(..1); // Only 1 signer

        get_validated_context(&e, &context, &insufficient_signers);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
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
            &create_rule_val_with_signers(&e, "other_rule"),
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

        // Add multiple rules
        let rule1 = add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "rule1"));
        let rule2 = add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "rule2"));
        let rule3 = add_context_rule(&e, &context_type, &create_rule_val_with_signers(&e, "rule3"));

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
