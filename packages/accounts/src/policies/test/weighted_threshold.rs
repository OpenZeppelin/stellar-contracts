#![cfg(test)]
use soroban_sdk::{auth::Context, testutils::Address as _, Address, Env, IntoVal, Map, Vec};

use crate::{policies::weighted_threshold::*, smart_account::storage::Signer};

#[test]
fn weighted_multisig_get_config() {
    let e = Env::default();
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let smart_account = Address::generate(&e);

    let mut weights = Map::new(&e);
    weights.set(Signer::Native(addr1), 100u32);
    weights.set(Signer::Native(addr2), 50u32);

    let contract = e.register(WeightedThresholdPolicy, (weights, 75u32, smart_account));
    let client = WeightedThresholdPolicyClient::new(&e, &contract);
    let config = client.get_config();
    assert_eq!(config.threshold, 75);
}

#[test]
fn weight_calculation() {
    let e = Env::default();
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let smart_account = Address::generate(&e);

    let mut weights = Map::new(&e);
    weights.set(Signer::Native(addr1.clone()), 100u32);
    weights.set(Signer::Native(addr2.clone()), 50u32);

    let contract = e.register(WeightedThresholdPolicy, (weights, 75u32, smart_account));
    let client = WeightedThresholdPolicyClient::new(&e, &contract);

    let signers = Vec::from_array(&e, [Signer::Native(addr1), Signer::Native(addr2)]);
    let total_weight = client.calculate_weight(&signers);

    assert_eq!(total_weight, 150);
}

#[test]
fn can_enforce_sufficient_weight() {
    let e = Env::default();
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let source = Address::generate(&e);
    let smart_account = Address::generate(&e);

    let mut weights = Map::new(&e);
    weights.set(Signer::Native(addr1.clone()), 100u32);
    weights.set(Signer::Native(addr2.clone()), 50u32);

    let contract = e.register(WeightedThresholdPolicy, (weights, 75u32, smart_account));
    let client = WeightedThresholdPolicyClient::new(&e, &contract);

    let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr1)]);
    let context_rule_signers = Vec::new(&e);

    let can_enforce = client.can_enforce(
        &source,
        &Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        }),
        &context_rule_signers,
        &authenticated_signers,
    );

    assert!(can_enforce);
}

#[test]
fn can_enforce_insufficient_weight() {
    let e = Env::default();
    let addr1 = Address::generate(&e);
    let addr2 = Address::generate(&e);
    let source = Address::generate(&e);
    let smart_account = Address::generate(&e);

    let mut weights = Map::new(&e);
    weights.set(Signer::Native(addr1.clone()), 100u32);
    weights.set(Signer::Native(addr2.clone()), 50u32);

    let contract = e.register(WeightedThresholdPolicy, (weights, 75u32, smart_account));
    let client = WeightedThresholdPolicyClient::new(&e, &contract);

    let authenticated_signers = Vec::from_array(&e, [Signer::Native(addr2)]);
    let context_rule_signers = Vec::new(&e);

    let can_enforce = client.can_enforce(
        &source,
        &Context::Contract(soroban_sdk::auth::ContractContext {
            contract: Address::generate(&e),
            fn_name: soroban_sdk::symbol_short!("test"),
            args: ().into_val(&e),
        }),
        &context_rule_signers,
        &authenticated_signers,
    );

    assert!(!can_enforce);
}
