use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events, Ledger},
    token::TokenClient,
    vec, Address, Env, FromVal, MuxedAddress, String, Symbol, Val, Vec,
};
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::{
    collect_fee, collect_fee_and_invoke, is_allowed_fee_token, is_fee_token_allowlist_enabled,
    set_allowed_fee_token, sweep_token, validate_expiration_ledger, validate_fee_bounds,
    FeeAbstractionApproval, FeeAbstractionStorageKey,
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
    // approval, transfer and collect fee
    assert_eq!(events.events().len(), 3);

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
    // approval, transfer and collect fee
    assert_eq!(events.events().len(), 3);

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

// Regression test for #840. The `expiration_ledger` parameter has no
// relationship to the real, on-chain allowance's actual expiration — that
// expiration was already fixed by whichever prior approve() call set it, and
// TokenClient doesn't even expose it as a readable value, only the amount via
// `allowance()`. Before the fix, this exact scenario (a genuinely valid,
// non-expired allowance of 100 until ledger 200, current ledger 101) panicked
// with `Error(Contract, #5006)` purely because the *unrelated* parameter
// (100) happened to be less than the current ledger sequence (101) — a false
// rejection against a real allowance that still had 99 ledgers of validity
// left. After the fix, the call succeeds: the SAC's own `transfer_from`
// expiry enforcement is what actually protects the real allowance, and it's
// not fooled by this parameter either way.
#[test]
fn collect_fee_with_lazy_approval_succeeds_regardless_of_expiration_ledger_param() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;

    let token_client = TokenClient::new(&e, &token_address);
    // approve enough (100 > max_fee_amount) till ledger 200 — genuinely valid
    token_client.approve(&user, &contract_address, &100, &200);

    e.ledger().set_sequence_number(101);

    e.as_contract(&contract_address, || {
        collect_fee(
            &e,
            &token_address,
            20,
            max_fee_amount,
            100, // < current ledger 101, previously caused a false #5006 panic
            &user,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });

    // The real allowance moved only by the fee actually spent (100 - 20 =
    // 80) — no re-approve happened, and the unrelated expiration_ledger
    // param neither blocked the call nor changed anything about the real
    // allowance's own expiration.
    let allowance = token_client.allowance(&user, &contract_address);
    assert_eq!(allowance, 80);

    let balance = token_client.balance(&recipient);
    assert_eq!(balance, 20);
}

// Second half of #840: proves the parameter is inert in the other
// direction too. An arbitrary, disconnected expiration_ledger — one that
// isn't even close to the real allowance's actual expiration of 200 —
// changes nothing about whether the call succeeds. There was never a
// tightening or loosening effect to preserve; the parameter simply doesn't
// reach the real allowance in the Lazy/sufficient-allowance branch.
#[test]
fn collect_fee_with_lazy_approval_ignores_unrelated_expiration_ledger_param() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    let max_fee_amount = 50;
    let token_client = TokenClient::new(&e, &token_address);

    // Real, on-chain allowance: 100 units, valid until ledger 200.
    token_client.approve(&user, &contract_address, &100, &200);
    e.ledger().set_sequence_number(101);

    e.as_contract(&contract_address, || {
        collect_fee(
            &e,
            &token_address,
            10,
            max_fee_amount,
            9999, // arbitrary, unrelated to the real allowance's expiration of 200
            &user,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });

    let remaining = token_client.allowance(&user, &contract_address);
    assert_eq!(remaining, 90, "only the transfer_from spend (10) should move the real allowance");
}

#[test]
#[should_panic(expected = "Error(Contract, #5005)")]
fn collect_fee_panics_invalid_user() {
    let e = Env::default();

    let contract_address = e.register(MockContract, ());
    let user = Address::generate(&e);
    let token_address = e.register(MockToken, (user.clone(),));
    let recipient = Address::generate(&e);

    e.as_contract(&contract_address, || {
        collect_fee(
            &e,
            &token_address,
            20,
            50,
            100,
            &contract_address,
            &recipient,
            FeeAbstractionApproval::Lazy,
        );
    });
}

#[test]
fn collect_fee_and_invoke_success() {
    let e = Env::default();
    e.mock_all_auths();

    let user = Address::generate(&e);
    let token = e.register(MockToken, (user.clone(),));
    let fee_recipient = Address::generate(&e);

    let contract_address = e.register(MockContract, ());

    let current_ledger = e.ledger().sequence();
    let target_contract = e.register(MockTarget, ());
    let target_fn = Symbol::new(&e, "greet");
    let target_args: Vec<Val> = vec![&e];

    let greeting = e.as_contract(&contract_address, || {
        collect_fee_and_invoke(
            &e,
            &token,
            20,
            50,
            current_ledger + 10,
            &target_contract,
            &target_fn,
            &target_args,
            &user,
            &fee_recipient,
            FeeAbstractionApproval::Lazy,
        )
    });
    assert_eq!(String::from_val(&e, &greeting), String::from_str(&e, "hello"));

    let events = e.events().all();
    assert_eq!(events.events().len(), 4);
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
    assert_eq!(events.events().len(), 2);
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

#[test]
#[should_panic(expected = "Error(Contract, #5006)")]
fn validate_expiration_ledger_past() {
    let e = Env::default();
    e.ledger().set_sequence_number(10);
    validate_expiration_ledger(&e, 9);
}

// ################## TOKEN SWEEPING TESTS ##################

#[test]
fn sweep_token_success() {
    let e = Env::default();
    e.mock_all_auths();

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
    assert_eq!(events.events().len(), 2);

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
