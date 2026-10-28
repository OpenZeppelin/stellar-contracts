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

// Fixed addresses, so that `create_test_weights` and `create_test_context_rule`
// describe the same two rule signers.
fn create_test_addresses(e: &Env) -> (Address, Address) {
    (
        Address::from_str(e, "GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQDZ7H"),
        Address::from_str(e, "GABAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEJXA"),
    )
}

fn create_test_weights(e: &Env) -> (Map<Signer, u32>, Address, Address) {
    let (addr1, addr2) = create_test_addresses(e);

    let mut weights = Map::new(e);
    weights.set(Signer::Delegated(addr1.clone()), 100u32);
    weights.set(Signer::Delegated(addr2.clone()), 50u32);

    (weights, addr1, addr2)
}

fn create_test_context_rule(e: &Env) -> ContextRule {
    let (addr1, addr2) = create_test_addresses(e);
    create_test_context_rule_with_signers(
        e,
        Vec::from_array(e, [Signer::Delegated(addr1), Signer::Delegated(addr2)]),
    )
}

fn create_test_context_rule_with_signers(e: &Env, signers: Vec<Signer>) -> ContextRule {
    let policies = Vec::new(e);
    ContextRule {
        id: 1,
        context_type: ContextRuleType::Default,
        name: soroban_sdk::String::from_str(e, "test_rule"),
        signers,
        signer_ids: Vec::new(e),
        policies,
        policy_ids: Vec::new(e),
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

        // Verify install event was emitted
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3214)")]
fn install_already_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();
    let (weights, _, _) = create_test_weights(&e);
    let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
    let context_rule = create_test_context_rule(&e);

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        install(&e, &params, &context_rule, &smart_account);
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
        // `install` rejects weights that can overflow, so store them directly
        // to exercise the overflow check in `calculate_weight`
        let context_rule = create_test_context_rule(&e);
        e.storage().persistent().set(
            &WeightedThresholdStorageKey::AccountContext(smart_account.clone(), context_rule.id),
            &params,
        );

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

        assert_eq!(e.events().all().events().len(), 1)
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
        assert_eq!(e.events().all().events().len(), 1);
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
        let signer1 = Signer::Delegated(Address::generate(&e));
        let signer2 = Signer::Delegated(Address::generate(&e));
        let mut weights = Map::new(&e);
        // Create weights that will overflow when added together
        weights.set(signer1.clone(), u32::MAX);
        weights.set(signer2.clone(), 1u32);

        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 100 };
        let context_rule =
            create_test_context_rule_with_signers(&e, Vec::from_array(&e, [signer1, signer2]));

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
        assert_eq!(e.events().all().events().len(), 1);
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

        // Verify uninstall event
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn uninstall_not_installed_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

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
        // Reduce signer1's weight from 100 to 10, making total weight 60
        // (10+50) This makes threshold 75 unreachable
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

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn install_threshold_reachable_only_with_non_rule_signer_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (addr1, addr2) = create_test_addresses(&e);
        let mut weights = Map::new(&e);
        weights.set(Signer::Delegated(addr1), 1u32);
        weights.set(Signer::Delegated(addr2), 1u32);
        // Not a signer of the rule, so this weight can never be authenticated
        weights.set(Signer::Delegated(Address::generate(&e)), 5u32);

        // Total configured weight is 7, but the rule's signers can only reach 2
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 3 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
fn install_ignores_non_rule_signer_weight() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (mut weights, _, _) = create_test_weights(&e);
        weights.set(Signer::Delegated(Address::generate(&e)), 500u32);

        // Reachable by the rule's signers (100 + 50), so install succeeds
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 150 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);

        assert_eq!(get_threshold(&e, context_rule.id, &smart_account), 150);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_threshold_counts_only_rule_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (mut weights, _, _) = create_test_weights(&e);
        weights.set(Signer::Delegated(Address::generate(&e)), 500u32);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 75 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        // 200 <= 650 configured, but the rule's signers can only reach 150
        set_threshold(&e, 200, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_signer_weight_counts_only_rule_signers_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 150 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        // A weight for a signer outside the rule does not add reachable
        // weight...
        set_signer_weight(
            &e,
            &Signer::Delegated(Address::generate(&e)),
            500,
            &context_rule,
            &smart_account,
        );
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        // ...so lowering a rule signer's weight still makes 150 unreachable
        let (_, addr1, _) = create_test_weights(&e);
        set_signer_weight(&e, &Signer::Delegated(addr1), 10, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3212)")]
fn install_non_rule_signer_weight_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let addr1 = Address::generate(&e);
        let mut weights = Map::new(&e);
        weights.set(Signer::Delegated(addr1.clone()), u32::MAX);
        // Not a rule signer yet, but could be added to the rule later
        weights.set(Signer::Delegated(Address::generate(&e)), 1u32);

        let params =
            WeightedThresholdAccountParams { signer_weights: weights, threshold: u32::MAX };
        let context_rule = create_test_context_rule_with_signers(
            &e,
            Vec::from_array(&e, [Signer::Delegated(addr1)]),
        );

        install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3212)")]
fn set_signer_weight_non_rule_signer_overflow_fails() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let smart_account = Address::generate(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let (weights, _, _) = create_test_weights(&e);
        let params = WeightedThresholdAccountParams { signer_weights: weights, threshold: 150 };
        let context_rule = create_test_context_rule(&e);

        install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context_rule = create_test_context_rule(&e);
        set_signer_weight(
            &e,
            &Signer::Delegated(Address::generate(&e)),
            u32::MAX,
            &context_rule,
            &smart_account,
        );
    });
}
