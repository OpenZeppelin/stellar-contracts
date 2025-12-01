use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events},
    Address, Env,
};

use crate::{
    check_allowed_fee_token, emit_fee_collected, emit_forward_executed,
    is_fee_token_allowlist_enabled, set_allowed_fee_token, sweep_token, validate_fee_bounds,
    FeeAbstractionStorageKey,
};

#[contract]
struct MockContract;

#[contract]
struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn balance(e: Env, _id: Address) -> i128 {
        e.storage().persistent().get(&symbol_short!("balance")).unwrap_or(1000)
    }

    pub fn transfer(_e: Env, _from: Address, _to: Address, _amount: i128) {}
}

// ################## FEE TOKEN ALLOWLIST TESTS ##################

#[test]
fn test_allowlist_disabled_by_default() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        assert!(!is_fee_token_allowlist_enabled(&e));
    });
}

#[test]
fn test_set_allowed_fee_token() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &token, true);

        // Should not panic
        check_allowed_fee_token(&e, &token);

        // Disallow the token
        set_allowed_fee_token(&e, &token, false);
    });

    // Should emit 2 events (2 token allowlist updates)
    let events = e.events().all();
    assert_eq!(events.len(), 2);
}

#[test]
fn test_swap_and_pop_removal_updates_mappings() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Allow three tokens -> indices 0,1,2
        set_allowed_fee_token(&e, &token1, true);
        set_allowed_fee_token(&e, &token2, true);
        set_allowed_fee_token(&e, &token3, true);

        // Remove the middle token (index 1). This should trigger swap-and-pop,
        // moving token3 from index 2 to index 1 and updating its TokenIndex.
        set_allowed_fee_token(&e, &token2, false);

        // token3 is now at index 1
        let i: u32 = e
            .storage()
            .persistent()
            .get(&FeeAbstractionStorageKey::TokenIndex(token3.clone()))
            .unwrap();
        assert_eq!(i, 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5001)")]
fn test_allowing_already_allowed_token_panics() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &token, true);
        // Second allow should panic with FeeTokenAlreadyAllowed
        set_allowed_fee_token(&e, &token, true);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn test_disallowed_token_panics() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &Address::generate(&e), true);
        // Token not allowed, should panic
        check_allowed_fee_token(&e, &token);
    });
}

#[test]
fn test_allowlist_disabled_allows_all_tokens() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Allowlist disabled when no tokens are allowed
        // Should not panic even though token is not explicitly allowed
        check_allowed_fee_token(&e, &token);
    });
}

// ################## VALIDATION TESTS ##################

#[test]
fn test_validate_fee_bounds_success() {
    let e = Env::default();
    validate_fee_bounds(&e, 100, 100);
    validate_fee_bounds(&e, 50, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn test_validate_fee_bounds_exceeds_max() {
    let e = Env::default();
    validate_fee_bounds(&e, 101, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn test_validate_fee_bounds_zero() {
    let e = Env::default();
    validate_fee_bounds(&e, 0, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn test_validate_fee_bounds_neg() {
    let e = Env::default();
    validate_fee_bounds(&e, 0, -1);
}

// ################## TOKEN SWEEPING TESTS ##################

#[test]
fn test_sweep_token_success() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token_address = e.register(MockToken, ());
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        let amount = sweep_token(&e, &token_address, &recipient);
        assert_eq!(amount, 1000);
    });

    let events = e.events().all();
    assert_eq!(events.len(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #5004)")]
fn test_sweep_token_no_balance() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let recipient = Address::generate(&e);

    let token_address = e.register(MockToken, ());
    e.as_contract(&token_address, || {
        e.storage().persistent().set(&symbol_short!("balance"), &0i128);
    });

    e.as_contract(&contract_address, || {
        sweep_token(&e, &token_address, &recipient);
    });
}

// ################## EVENT EMISSION TESTS ##################

#[test]
fn test_emit_fee_collected() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let collector = Address::generate(&e);
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        emit_fee_collected(&e, &user, &collector, &token, 100);
    });

    let events = e.events().all();
    assert_eq!(events.len(), 1);
}

#[test]
fn test_emit_forward_executed() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let target_contract = Address::generate(&e);
    let target_fn = symbol_short!("test");
    let target_args = soroban_sdk::vec![&e];

    e.as_contract(&contract_address, || {
        emit_forward_executed(&e, &user, &target_contract, &target_fn, &target_args);
    });

    let events = e.events().all();
    assert_eq!(events.len(), 1);
}
