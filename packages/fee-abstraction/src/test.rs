use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    token::TokenClient,
    vec, Address, Env, FromVal, MuxedAddress, String, Symbol, Val, Vec,
};
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::{
    auth_user_and_invoke, collect_fee, is_allowed_fee_token, is_fee_token_allowlist_enabled,
    set_allowed_fee_token, sweep_token, validate_fee_bounds, FeeAbstractionApproval,
    FeeAbstractionStorageKey,
};

#[contract]
struct MockContract;

#[contract]
pub struct MockTarget;

#[contractimpl]
impl MockTarget {
    pub fn greet(e: Env) -> String {
        String::from_str(&e, "hello")
    }
}

#[contract]
struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn __constructor(e: Env, user: Address) {
        Base::mint(&e, &user, 1_000);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for MockToken {
    type ContractType = Base;
}

#[test]
fn collect_fee_with_eager_approval_overwrites_allowance() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;

    let token_client = TokenClient::new(&e, &token_address);
    // approve initially > max_fee_amount, but it will be overwritten
    token_client.approve(&user, &contract_address, &60, &100);

    e.as_contract(&contract_address, || {
        // approve 50, spend 20
        collect_fee(
            &e,
            &token_address,
            20,
            max_fee_amount,
            100,
            &user,
            &recipient,
            FeeAbstractionApproval::Eager,
        );
    });

    let events = e.events().all();
    // approval, trnasfer and collect fee
    assert_eq!(events.len(), 3);

    let allowance = token_client.allowance(&user, &contract_address);
    assert_eq!(allowance, 30);

    let balance = token_client.balance(&recipient);
    assert_eq!(balance, 20);
}

#[test]
fn collect_fee_with_lazy_approval_no_previous() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;

    let token_client = TokenClient::new(&e, &token_address);
    // no previous approvals

    e.as_contract(&contract_address, || {
        // approve 50, spend 20
        collect_fee(
            &e,
            &token_address,
            20,
            max_fee_amount,
            100,
            &user,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });

    let events = e.events().all();
    // approval, trnasfer and collect fee
    assert_eq!(events.len(), 3);

    let allowance = token_client.allowance(&user, &contract_address);
    assert_eq!(allowance, 30);

    let balance = token_client.balance(&recipient);
    assert_eq!(balance, 20);
}

#[test]
fn collect_fee_with_lazy_approval_higher_previous() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;

    let token_client = TokenClient::new(&e, &token_address);
    // approve initially > max_fee_amount
    token_client.approve(&user, &contract_address, &60, &100);

    e.as_contract(&contract_address, || {
        // no approval, only spend 20
        collect_fee(
            &e,
            &token_address,
            20,
            max_fee_amount,
            100,
            &user,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });

    let allowance = token_client.allowance(&user, &contract_address);
    assert_eq!(allowance, 40);

    let balance = token_client.balance(&recipient);
    assert_eq!(balance, 20);
}

#[test]
fn collect_fee_with_lazy_approval_lower_previous() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;

    let token_client = TokenClient::new(&e, &token_address);
    // approve initially < max_fee_amount
    token_client.approve(&user, &contract_address, &30, &100);

    e.as_contract(&contract_address, || {
        // approve 50 by overwriting the previous 30 and spend 20
        collect_fee(
            &e,
            &token_address,
            20,
            max_fee_amount,
            100,
            &user,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });

    let allowance = token_client.allowance(&user, &contract_address);
    assert_eq!(allowance, 30);

    let balance = token_client.balance(&recipient);
    assert_eq!(balance, 20);
}

#[test]
#[should_panic(expected = "Error(Contract, #5005)")]
fn collect_fee_panics_invalid_fee_recipient() {
    let e = Env::default();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));

    e.as_contract(&contract_address, || {
        collect_fee(&e, &token_address, 20, 50, 100, &user, &user, FeeAbstractionApproval::Lazy);
    });
}

#[test]
fn auth_user_and_invoke_success() {
    let e = Env::default();
    e.mock_all_auths();

    let token = Address::generate(&e);
    let user = Address::generate(&e);

    let contract_address = e.register(MockContract, ());

    let current_ledger = e.ledger().sequence();
    let target_contract = e.register(MockTarget, ());
    let target_fn = Symbol::new(&e, "greet");
    let target_args: Vec<Val> = vec![&e];

    let greeting = e.as_contract(&contract_address, || {
        auth_user_and_invoke(
            &e,
            &token,
            50,
            current_ledger + 10,
            &target_contract,
            &target_fn,
            &target_args,
            &user,
        )
    });
    assert_eq!(String::from_val(&e, &greeting), String::from_str(&e, "hello"));

    let events = e.events().all();
    assert_eq!(events.len(), 1);
}

// ################## FEE TOKEN ALLOWLIST TESTS ##################

#[test]
fn allowlist_disabled_by_default() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        assert!(!is_fee_token_allowlist_enabled(&e));
    });
}

#[test]
fn set_allowed_fee_token_success() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &token, true);

        // Should be allowed
        assert!(is_allowed_fee_token(&e, &token));

        // Disallow the token
        set_allowed_fee_token(&e, &token, false);
    });

    // Should emit 2 events (2 token allowlist updates)
    let events = e.events().all();
    assert_eq!(events.len(), 2);
}

#[test]
fn swap_and_pop_removal_updates_mappings() {
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
fn allowing_already_allowed_token_panics() {
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
fn not_allowed_token() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &Address::generate(&e), true);
        // Token not allowed
        assert!(!is_allowed_fee_token(&e, &token));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #5000)")]
fn not_allowed_token_panics() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        set_allowed_fee_token(&e, &Address::generate(&e), true);
        collect_fee(
            &e,
            &token,
            20,
            50,
            100,
            &Address::generate(&e),
            &Address::generate(&e),
            FeeAbstractionApproval::Eager,
        );
    });
}

#[test]
fn allowlist_disabled_allows_all_tokens() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_address, || {
        // Allowlist disabled when no tokens are allowed
        // Should return true even though token is not explicitly allowed
        assert!(is_allowed_fee_token(&e, &token));
    });
}

// ################## VALIDATION TESTS ##################

#[test]
fn validate_fee_bounds_success() {
    let e = Env::default();
    validate_fee_bounds(&e, 100, 100);
    validate_fee_bounds(&e, 50, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn validate_fee_bounds_exceeds_max() {
    let e = Env::default();
    validate_fee_bounds(&e, 101, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn validate_fee_bounds_zero() {
    let e = Env::default();
    validate_fee_bounds(&e, 0, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5003)")]
fn validate_fee_bounds_neg() {
    let e = Env::default();
    validate_fee_bounds(&e, 0, -1);
}

// ################## TOKEN SWEEPING TESTS ##################

#[test]
fn sweep_token_success() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let recipient = Address::generate(&e);

    let token_address = e.register(MockToken, (contract_address.clone(),));
    let token_client = TokenClient::new(&e, &token_address);

    let balance = token_client.balance(&contract_address);

    e.as_contract(&contract_address, || {
        sweep_token(&e, &token_address, &recipient);
    });

    // transfer + token swept
    let events = e.events().all();
    assert_eq!(events.len(), 2);

    assert_eq!(token_client.balance(&recipient), balance);
    assert_eq!(token_client.balance(&contract_address), 0i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #5004)")]
fn sweep_token_no_balance() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    let token_address = e.register(MockToken, (Address::generate(&e),));
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        sweep_token(&e, &token_address, &recipient);
    });
}
