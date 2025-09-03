#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::Context,
    contract,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Vec,
};

use crate::policies::simple_threshold::*;

#[contract]
struct MockContract;

fn create_test_signers(e: &Env) -> (Address, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);
    let addr3 = Address::generate(e);

    (addr1, addr2, addr3)
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, _, _) = create_test_signers(&e);
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);

        assert_eq!(get_threshold(&e, &smart_account), 2);
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
        let params = SimpleThresholdInstallParams { threshold: 0, signers_count: 3 }; // Invalid

        install(&e, &params, &smart_account);
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
        get_threshold(&e, &smart_account);
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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);

        let authenticated_signers =
            Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr1)]);
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

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
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        let result = can_enforce(
            &e,
            &context,
            &context_rule_signers,
            &authenticated_signers,
            &smart_account,
        );

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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);

        authenticated_signers
    });

    e.as_contract(&address, || {
        let context_rule_signers = Vec::new(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        enforce(&e, &context, &context_rule_signers, &authenticated_signers, &smart_account);

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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 2 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 3, 3, &smart_account);
        assert_eq!(get_threshold(&e, &smart_account), 3);
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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 0, 3, &smart_account); // Invalid threshold
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
        let params = SimpleThresholdInstallParams { threshold: 2, signers_count: 3 };

        install(&e, &params, &smart_account);

        // Verify it's installed
        assert_eq!(get_threshold(&e, &smart_account), 2);
    });

    e.as_contract(&address, || {
        uninstall(&e, &smart_account);
    });
}
