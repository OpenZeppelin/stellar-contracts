#![cfg(test)]
use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::rwa::utils::token_binder::storage::{
    bind_token, get_token_by_index, get_token_index, is_token_bound, linked_token_count,
    linked_tokens, unbind_token,
};

#[contract]
struct MockContract;

#[test]
fn linked_token_count_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let count = linked_token_count(&e);
        assert_eq!(count, 0);
    });
}

#[test]
fn bind_single_token() {
    let e = Env::default();
    let token = Address::generate(&e);
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        bind_token(&e, &token);

        assert_eq!(linked_token_count(&e), 1);
        assert!(is_token_bound(&e, &token));
        assert_eq!(get_token_by_index(&e, 0), token);
        assert_eq!(get_token_index(&e, &token), 0);
    });
}

#[test]
fn bind_multiple_tokens() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        assert_eq!(linked_token_count(&e), 3);
        assert!(is_token_bound(&e, &token1));
        assert!(is_token_bound(&e, &token2));
        assert!(is_token_bound(&e, &token3));

        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token2);
        assert_eq!(get_token_by_index(&e, 2), token3);

        assert_eq!(get_token_index(&e, &token1), 0);
        assert_eq!(get_token_index(&e, &token2), 1);
        assert_eq!(get_token_index(&e, &token3), 2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #331)")]
fn bind_duplicate_token() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token);
        bind_token(&e, &token);
    });
}

#[test]
fn unbind_single_token() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token);
        assert_eq!(linked_token_count(&e), 1);

        unbind_token(&e, &token);
        assert_eq!(linked_token_count(&e), 0);
        assert!(!is_token_bound(&e, &token));
    });
}

#[test]
fn unbind_middle_token_swap_remove() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        unbind_token(&e, &token2);

        assert_eq!(linked_token_count(&e), 2);
        assert!(is_token_bound(&e, &token1));
        assert!(!is_token_bound(&e, &token2));
        assert!(is_token_bound(&e, &token3));

        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token3);

        assert_eq!(get_token_index(&e, &token1), 0);
        assert_eq!(get_token_index(&e, &token3), 1);
    });
}

#[test]
fn unbind_last_token() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        unbind_token(&e, &token3);

        assert_eq!(linked_token_count(&e), 2);
        assert!(is_token_bound(&e, &token1));
        assert!(is_token_bound(&e, &token2));
        assert!(!is_token_bound(&e, &token3));

        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")]
fn unbind_nonexistent_token() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_id, || {
        unbind_token(&e, &token);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")]
fn get_token_by_invalid_index() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        get_token_by_index(&e, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")]
fn get_token_index_nonexistent() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_id, || {
        get_token_index(&e, &token);
    });
}

#[test]
fn is_token_bound_false() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    let result = e.as_contract(&contract_id, || is_token_bound(&e, &token));
    assert!(!result);
}

#[test]
fn linked_tokens_empty() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());

    e.as_contract(&contract_id, || {
        let tokens = linked_tokens(&e);
        assert_eq!(tokens.len(), 0);
    });
}

#[test]
fn linked_tokens_multiple() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    let tokens = e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        linked_tokens(&e)
    });

    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens.get(0).unwrap(), token1);
    assert_eq!(tokens.get(1).unwrap(), token2);
    assert_eq!(tokens.get(2).unwrap(), token3);
}

#[test]
fn complex_bind_unbind_sequence() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);
    let token4 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);
        assert_eq!(linked_token_count(&e), 3);

        unbind_token(&e, &token2);
        assert_eq!(linked_token_count(&e), 2);
        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token3);

        bind_token(&e, &token4);
        assert_eq!(linked_token_count(&e), 3);
        assert_eq!(get_token_by_index(&e, 2), token4);

        unbind_token(&e, &token1);
        assert_eq!(linked_token_count(&e), 2);
        assert_eq!(get_token_by_index(&e, 0), token4);
        assert_eq!(get_token_by_index(&e, 1), token3);
    });
}

#[test]
fn bind_unbind_all_tokens() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);
        assert_eq!(linked_token_count(&e), 3);

        unbind_token(&e, &token1);
        unbind_token(&e, &token2);
        unbind_token(&e, &token3);

        assert_eq!(linked_token_count(&e), 0);
        assert!(!is_token_bound(&e, &token1));
        assert!(!is_token_bound(&e, &token2));
        assert!(!is_token_bound(&e, &token3));
    });
}

#[test]
fn rebind_after_unbind() {
    let e = Env::default();
    let contract_id = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&contract_id, || {
        bind_token(&e, &token);
        assert!(is_token_bound(&e, &token));
        assert_eq!(get_token_index(&e, &token), 0);

        unbind_token(&e, &token);
        assert!(!is_token_bound(&e, &token));

        bind_token(&e, &token);
        assert!(is_token_bound(&e, &token));
        assert_eq!(get_token_index(&e, &token), 0);
        assert_eq!(linked_token_count(&e), 1);
    });
}
