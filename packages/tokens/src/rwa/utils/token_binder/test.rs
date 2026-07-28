extern crate std;

use soroban_sdk::{
    contract,
    testutils::{Address as _, Events},
    vec, Address, Env, Vec,
};

use crate::rwa::utils::token_binder::{
    storage::{
        bind_token, bind_tokens, is_token_bound, linked_token_count, linked_tokens, unbind_token,
        TokenBinderStorageKey,
    },
    MAX_TOKENS,
};

#[contract]
struct MockContract;

#[test]
fn linked_token_count_empty() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let count = linked_token_count(&e);
        assert_eq!(count, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #332)")]
fn bind_token_max_tokens_reached() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();
    let address = e.register(MockContract, ());
    e.as_contract(&address, || {
        let mut tokens: Vec<Address> = Vec::new(&e);
        for _ in 0..MAX_TOKENS {
            tokens.push_back(Address::generate(&e));
        }
        e.storage().persistent().set(&TokenBinderStorageKey::Tokens, &tokens);

        // Next bind should panic with MaxTokensReached
        let extra = Address::generate(&e);
        bind_token(&e, &extra);
    });
}

#[test]
fn bind_tokens_appends_in_order() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let tokens = e.as_contract(&address, || {
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..10u32 {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);

        // verify
        assert_eq!(linked_tokens(&e), batch);
        // one TokenBound per token
        assert_eq!(e.events().all().events().len(), 10);

        linked_tokens(&e)
    });

    assert_eq!(tokens.len(), 10);
}

#[test]
fn bind_tokens_full_capacity_in_one_call() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // A single batch can bind up to the full capacity.
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..MAX_TOKENS {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);

        assert_eq!(linked_token_count(&e), MAX_TOKENS);
        assert_eq!(linked_tokens(&e), batch);
        assert!(is_token_bound(&e, &batch.get(MAX_TOKENS / 2).unwrap()));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #334)")]
fn bind_tokens_duplicates_should_panic() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let t1 = Address::generate(&e);
    let t2 = Address::generate(&e);

    e.as_contract(&address, || {
        let mut batch: Vec<Address> = Vec::new(&e);
        batch.push_back(t1.clone());
        batch.push_back(t2.clone());
        batch.push_back(t1.clone()); // duplicate

        bind_tokens(&e, &batch);
    });
}

#[test]
fn bind_single_token() {
    let e = Env::default();
    let token = Address::generate(&e);
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        bind_token(&e, &token);

        assert_eq!(linked_token_count(&e), 1);
        assert!(is_token_bound(&e, &token));
        assert_eq!(linked_tokens(&e), vec![&e, token.clone()]);
        assert_eq!(e.events().all().events().len(), 1);
    });
}

#[test]
fn bind_multiple_tokens() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        assert_eq!(linked_token_count(&e), 3);
        assert!(is_token_bound(&e, &token1));
        assert!(is_token_bound(&e, &token2));
        assert!(is_token_bound(&e, &token3));

        assert_eq!(linked_tokens(&e), vec![&e, token1.clone(), token2.clone(), token3.clone()]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #331)")]
fn bind_duplicate_token() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token);
        bind_token(&e, &token);
    });
}

#[test]
fn unbind_single_token() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token);
        assert_eq!(linked_token_count(&e), 1);

        unbind_token(&e, &token);
        assert_eq!(linked_token_count(&e), 0);
        assert!(!is_token_bound(&e, &token));
        // 1 TokenBound + 1 TokenUnbound
        assert_eq!(e.events().all().events().len(), 2);
    });
}

#[test]
fn unbind_middle_token_swap_remove() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        unbind_token(&e, &token2);

        assert_eq!(linked_token_count(&e), 2);
        assert!(is_token_bound(&e, &token1));
        assert!(!is_token_bound(&e, &token2));
        assert!(is_token_bound(&e, &token3));

        // The last token (token3) filled the removed slot
        assert_eq!(linked_tokens(&e), vec![&e, token1.clone(), token3.clone()]);
    });
}

#[test]
fn unbind_last_token() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);

        unbind_token(&e, &token3);

        assert_eq!(linked_token_count(&e), 2);
        assert!(is_token_bound(&e, &token1));
        assert!(is_token_bound(&e, &token2));
        assert!(!is_token_bound(&e, &token3));

        assert_eq!(linked_tokens(&e), vec![&e, token1.clone(), token2.clone()]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")]
fn unbind_nonexistent_token() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&address, || {
        unbind_token(&e, &token);
    });
}

#[test]
fn is_token_bound_false() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    let result = e.as_contract(&address, || is_token_bound(&e, &token));
    assert!(!result);
}

#[test]
fn linked_tokens_empty() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let tokens = linked_tokens(&e);
        assert_eq!(tokens.len(), 0);
    });
}

#[test]
fn linked_tokens_multiple() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    let tokens = e.as_contract(&address, || {
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
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);
    let token4 = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token1);
        bind_token(&e, &token2);
        bind_token(&e, &token3);
        assert_eq!(linked_token_count(&e), 3);

        unbind_token(&e, &token2);
        assert_eq!(linked_tokens(&e), vec![&e, token1.clone(), token3.clone()]);

        bind_token(&e, &token4);
        assert_eq!(linked_tokens(&e), vec![&e, token1.clone(), token3.clone(), token4.clone()]);

        unbind_token(&e, &token1);
        // The last token (token4) filled the removed slot
        assert_eq!(linked_tokens(&e), vec![&e, token4.clone(), token3.clone()]);
    });
}

#[test]
fn bind_unbind_all_tokens() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token1 = Address::generate(&e);
    let token2 = Address::generate(&e);
    let token3 = Address::generate(&e);

    e.as_contract(&address, || {
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
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&address, || {
        bind_token(&e, &token);
        assert!(is_token_bound(&e, &token));

        unbind_token(&e, &token);
        assert!(!is_token_bound(&e, &token));

        bind_token(&e, &token);
        assert!(is_token_bound(&e, &token));
        assert_eq!(linked_tokens(&e), vec![&e, token.clone()]);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #332)")]
fn bind_tokens_exceeding_capacity_panics() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let target_len = MAX_TOKENS + 1; // strictly greater than capacity
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..target_len {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #331)")]
fn bind_tokens_already_bound_in_storage_should_panic() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Pre-bind a token T
        let t = Address::generate(&e);
        bind_token(&e, &t);

        // Batch includes T but has no internal duplicates
        let mut batch: Vec<Address> = Vec::new(&e);
        batch.push_back(Address::generate(&e));
        batch.push_back(t.clone());
        batch.push_back(Address::generate(&e));

        bind_tokens(&e, &batch);
    });
}

#[test]
fn bind_tokens_appends_after_existing() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Pre-bind a handful of single tokens
        for _ in 0..5u32 {
            bind_token(&e, &Address::generate(&e));
        }
        assert_eq!(linked_token_count(&e), 5);

        // A batch appends after the existing entries, in order
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..10u32 {
            batch.push_back(Address::generate(&e));
        }
        bind_tokens(&e, &batch);

        let all = linked_tokens(&e);
        assert_eq!(all.len(), 15);
        assert_eq!(all.slice(5..), batch);
    });
}
