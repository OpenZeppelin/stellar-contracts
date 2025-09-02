#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::Context,
    contract,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Map, Vec,
};

use crate::policies::weighted_threshold::*;

#[contract]
struct MockContract;

fn create_test_weights(e: &Env) -> (Map<Signer, u32>, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);

    let mut weights = Map::new(e);
    weights.set(Signer::Native(addr1.clone()), 100u32);
    weights.set(Signer::Native(addr2.clone()), 50u32);

    (weights, addr1, addr2)
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);

        assert_eq!(get_thershold(&e, &smart_account), 75);
        let stored_weights = get_signer_weights(&e, &smart_account);
        assert_eq!(stored_weights.len(), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn install_zero_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams {
            signer_weights: weights,
            threshold: 0, // Invalid
        };

        install(&e, &params, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn install_zero_weight_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let mut weights = Map::new(&e);
        weights.set(Signer::Native(Address::generate(&e)), 0u32); // Invalid weight
        weights.set(Signer::Native(Address::generate(&e)), 50u32);

        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn install_threshold_exceeds_total_weight_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams {
            signer_weights: weights,
            threshold: 200, // Exceeds total weight of 150
        };

        install(&e, &params, &smart_account);
    });
}

#[test]
fn calculate_weight_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, addr1, addr2) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);

        let signers = Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
        let total_weight = calculate_weight(&e, &signers, &smart_account);

        assert_eq!(total_weight, 150);
    });
}

#[test]
fn calculate_weight_partial_signers() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, addr1, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);

        let signers = Vec::from_array(&e, [Signer::Native(addr1)]);
        let total_weight = calculate_weight(&e, &signers, &smart_account);

        assert_eq!(total_weight, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn calculate_weight_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.as_contract(&address, || {
        let signers = Vec::from_array(&e, [Signer::Native(Address::generate(&e))]);
        calculate_weight(&e, &signers, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn calculate_weight_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let (addr1, addr2) = e.as_contract(&address, || {
        let addr1 = Address::generate(&e);
        let addr2 = Address::generate(&e);

        let mut weights = Map::new(&e);
        weights.set(Signer::Native(addr1.clone()), u32::MAX);
        weights.set(Signer::Native(addr2.clone()), 1u32);

        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 100 };
        install(&e, &params, &smart_account);

        (addr1, addr2)
    });

    e.as_contract(&address, || {
        // Try to calculate weight with signers that will cause overflow
        let signers = Vec::from_array(
            &e,
            [
                Signer::Native(addr1), // This will have weight u32::MAX
                Signer::Native(addr2), // This will have weight 1
            ],
        );
        calculate_weight(&e, &signers, &smart_account);
    });
}

#[test]
fn can_enforce_sufficient_weight() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, addr1, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

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

        assert!(result);
    });
}

#[test]
fn can_enforce_insufficient_weight() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, addr2) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr2)]);
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
        let (weights, addr1, _) = create_test_weights(&e);
        let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr1)]);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

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

        assert_eq!(e.events().all().len(), 1)
    });
}

#[test]
fn set_threshold_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 100, &smart_account);
        assert_eq!(get_thershold(&e, &smart_account), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn set_threshold_zero_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        set_threshold(&e, 0, &smart_account); // Invalid threshold
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn install_math_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let mut weights = Map::new(&e);
        // Create weights that will overflow when added together
        weights.set(Signer::Native(Address::generate(&e)), u32::MAX);
        weights.set(Signer::Native(Address::generate(&e)), 1u32);

        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 100 };

        install(&e, &params, &smart_account);
    });
}

#[test]
fn set_signer_weight_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);
    });

    e.as_contract(&address, || {
        let new_signer = Signer::Native(Address::generate(&e));
        set_signer_weight(&e, &new_signer, 25, &smart_account);

        let updated_weights = get_signer_weights(&e, &smart_account);
        assert_eq!(updated_weights.get(new_signer).unwrap(), 25);
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdInstallParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &smart_account);

        // Verify it's installed
        assert_eq!(get_thershold(&e, &smart_account), 75);
    });

    e.as_contract(&address, || {
        uninstall(&e, &smart_account);
    });
}
