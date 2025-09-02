#![cfg(test)]
extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, Bytes, BytesN, Env, String, Symbol, Val, Vec,
};

use super::{
    enforce_policy,
    storage::{
        add_context_rule, get_context_rule, modify_context_rule, remove_context_rule, ContextRule,
        ContextRuleType, ContextRuleVal, Signer,
    },
};

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
        let mut new_rule_val = create_rule_val_with_policies(&e, "modified_rule");
        new_rule_val.valid_until = Some(future_sequence);

        let modified_rule = modify_context_rule(&e, rule.id, &new_rule_val);

        assert_eq!(modified_rule.id, rule.id);
        assert_eq!(modified_rule.name, String::from_str(&e, "modified_rule"));
        assert_eq!(modified_rule.signers.len(), 0);
        assert_eq!(modified_rule.policies.len(), 2);
        assert_eq!(modified_rule.valid_until, Some(future_sequence));

        // Verify it was actually stored
        let retrieved_rule = get_context_rule(&e, rule.id);
        assert_eq!(retrieved_rule.name, String::from_str(&e, "modified_rule"));
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
