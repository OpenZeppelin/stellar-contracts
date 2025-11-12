extern crate std;

use soroban_sdk::{
    auth::Context,
    contract,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Map, Vec,
};

use crate::{
    policies::weighted_threshold::*,
    smart_account::{ContextRule, ContextRuleType, Signer},
};

#[contract]
struct MockContract;

fn create_test_weights(e: &Env) -> (Map<Signer, u32>, Address, Address) {
    let addr1 = Address::generate(e);
    let addr2 = Address::generate(e);

    let mut weights = Map::new(e);
    weights.set(Signer::Delegated(addr1.clone()), 100u32);
    weights.set(Signer::Delegated(addr2.clone()), 50u32);

    (weights, addr1, addr2)
}

fn create_test_context_rule(e: &Env) -> ContextRule {
    let (_, addr1, addr2) = create_test_weights(e);
    let mut signers = Vec::new(e);
    signers.push_back(Signer::Delegated(addr1));
    signers.push_back(Signer::Delegated(addr2));
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
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 75);
        let stored_weights = get_signer_weights(&e, &context_rule, &smart_account);
        assert_eq!(stored_weights.len(), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn install_zero_threshold_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams {
            signer_weights: weights,
            threshold: 0, // Invalid
        };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn install_threshold_exceeds_total_weight_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams {
            signer_weights: weights,
            threshold: 200, // Exceeds total weight of 150
        };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let signers = Vec::from_array(&e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]);
        let total_weight = calculate_weight(&e, &signers, &context_rule, &smart_account);

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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);
        let total_weight = calculate_weight(&e, &signers, &context_rule, &smart_account);

        assert_eq!(total_weight, 100);
    });
}

#[test]
fn calculate_weight_signer_without_weight() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, addr1, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        // Create a signer that doesn't have weight assigned
        let unknown_signer = Address::generate(&e);
        let signers = Vec::from_array(
            &e,
            [
                Signer::Delegated(addr1),          // This has weight 100
                Signer::Delegated(unknown_signer), // This has no weight assigned
            ],
        );

        let total_weight = calculate_weight(&e, &signers, &context_rule, &smart_account);

        // Should only count addr1's weight (100), unknown_signer is skipped
        assert_eq!(total_weight, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn calculate_weight_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.as_contract(&address, || {
        let signers = Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]);
        let context_rule = create_test_context_rule(&e);
        calculate_weight(&e, &signers, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3212)")]
fn calculate_weight_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    let (addr1, addr2) = e.as_contract(&address, || {
        let addr1 = Address::generate(&e);
        let addr2 = Address::generate(&e);

        let mut weights = Map::new(&e);
        weights.set(Signer::Delegated(addr1.clone()), u32::MAX);
        weights.set(Signer::Delegated(addr2.clone()), 1u32);

        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 100 };
        let context_rule = create_test_context_rule(&e);
        install(&e, &params, &context_rule, &smart_account);

        (addr1, addr2)
    });

    e.as_contract(&address, || {
        // Try to calculate weight with signers that will cause overflow
        let signers = Vec::from_array(
            &e,
            [
                Signer::Delegated(addr1), // This will have weight u32::MAX
                Signer::Delegated(addr2), // This will have weight 1
            ],
        );
        let context_rule = create_test_context_rule(&e);
        calculate_weight(&e, &signers, &context_rule, &smart_account);
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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);

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
fn can_enforce_insufficient_weight() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, addr2) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(addr2)]);

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
        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(Address::generate(&e))]);
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
        let (weights, addr1, _) = create_test_weights(&e);
        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(addr1)]);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 100, &context_rule, &smart_account);
        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_threshold_zero_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_threshold(&e, 0, &context_rule, &smart_account); // Invalid threshold
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3212)")]
fn install_math_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let mut weights = Map::new(&e);
        // Create weights that will overflow when added together
        weights.set(Signer::Delegated(Address::generate(&e)), u32::MAX);
        weights.set(Signer::Delegated(Address::generate(&e)), 1u32);

        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 100 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        let new_signer = Signer::Delegated(Address::generate(&e));
        set_signer_weight(&e, &new_signer, 25, &context_rule, &smart_account);

        let updated_weights = get_signer_weights(&e, &context_rule, &smart_account);
        assert_eq!(updated_weights.get(new_signer).unwrap(), 25);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn set_threshold_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);

        // Try to set threshold without installing the policy first
        set_threshold(&e, 100, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn set_signer_weight_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        let new_signer = Signer::Delegated(Address::generate(&e));

        // Try to set signer weight without installing the policy first
        set_signer_weight(&e, &new_signer, 25, &context_rule, &smart_account);
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
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        // Verify it's installed
        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 75);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_threshold_unreachable_fails() {
    let e = Env::default();
    e.mock_all_auths();

    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_test_context_rule(&e);
    let (weights, _, _) = create_test_weights(&e);

    e.as_contract(&address, || {
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        // Try to set threshold higher than total weight (150)
        set_threshold(&e, 200, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_signer_weight_makes_threshold_unreachable_fails() {
    let e = Env::default();
    e.mock_all_auths();

    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_test_context_rule(&e);

    let (weights, signer1, _) = create_test_weights(&e);

    e.as_contract(&address, || {
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        // Reduce signer1's weight from 100 to 10, making total weight 60 (10+50)
        // This makes threshold 75 unreachable
        set_signer_weight(&e, &Signer::Delegated(signer1), 10, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn enforce_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (_, addr1, addr2) = create_test_weights(&e);
        let authenticated_signers =
            Vec::from_array(&e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]);
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        // Try to enforce without installing the policy first
        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3213)")]
fn enforce_threshold_not_met_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let (_, _, addr2) = create_test_weights(&e);
        // Only addr2 authenticated with weight 50, but threshold is 75
        let authenticated_signers = Vec::from_array(&e, [Signer::Delegated(addr2)]);
        let context_rule = create_test_context_rule(&e);

        let context = Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        });

        // Should fail because weight is 50 but threshold is 75
        enforce(&e, &context, &authenticated_signers, &context_rule, &smart_account);
    });
}
