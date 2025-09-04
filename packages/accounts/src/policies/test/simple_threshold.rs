#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::Context,
    contract,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Vec,
};

use crate::{
    policies::simple_threshold::*,
    smart_account::{ContextRule, ContextRuleType},
};

#[contract]
struct MockContract;

fn create_test_signers(e: &Env) -> (Address, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);
    let addr3 = Address::generate(e);

    (addr1, addr2, addr3)
}

fn create_test_context_rule(e: &Env) -> ContextRule {
    let (addr1, addr2, addr3) = create_test_signers(e);
    let mut signers = Vec::new(e);
    signers.push_back(Signer::Native(addr1));
    signers.push_back(Signer::Native(addr2));
    signers.push_back(Signer::Native(addr3));
    let policies = Vec::new(e);
    ContextRule {
        id: 1,
        context_type: ContextRuleType::Default,
        name: soroban_sdk::String::from_str(e, "test_rule"),
        signers,
        policies,
        valid_until: None,
    }
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        assert_eq!(get_threshold(&e, &context_rule, &smart_account), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2201)")]
fn install_zero_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 0 }; // Invalid
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2200)")]
fn smart_account_get_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        get_threshold(&e, &context_rule, &smart_account);
    });
}

#[test]
fn can_enforce_sufficient_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (addr1, addr2, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let authenticated_signers =
            Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result =
            can_enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        assert!(result);
    });
}

#[test]
fn can_enforce_insufficient_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (addr1, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr1)]);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result =
            can_enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        assert!(!result);
    });
}

#[test]
fn can_enforce_not_installed() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.as_contract(&address, || {
        let authenticated_signers = Vec::from_array(&e, [Signer::Native(Address::generate(&e))]);
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result =
            can_enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        assert!(!result);
    });
}

#[test]
fn enforce_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let authenticated_signers = e.as_contract(&address, || {
        let (addr1, addr2, _) = create_test_signers(&e);
        let authenticated_signers =
            Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        authenticated_signers
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);

        assert_eq!(e.events().all().len(), 1);
    });
}

#[test]
fn set_threshold_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 3, &context_rule, &smart_account);
        assert_eq!(get_threshold(&e, &context_rule, &smart_account), 3);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2201)")]
fn set_threshold_zero_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 0, &context_rule, &smart_account); // Invalid threshold
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdAccountParams { threshold: 2 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        // Verify it's installed
        assert_eq!(get_threshold(&e, &context_rule, &smart_account), 2);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        uninstall(&e, &context_rule, &smart_account);
    });
}
