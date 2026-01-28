use soroban_sdk::{contract, testutils::Address as _, Address, Env, Vec};

use super::storage::TokenBinderStorageKey;
use crate::rwa::utils::token_binder::{
    storage::{
        bind_token, bind_tokens, get_token_by_index, get_token_index, is_token_bound,
        linked_token_count, linked_tokens, unbind_token,
    },
    BUCKET_SIZE, MAX_TOKENS,
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
        e.storage().persistent().set(&TokenBinderStorageKey::TotalCount, &MAX_TOKENS);
        // Next bind should panic with MaxTokensReached
        let extra = Address::generate(&e);
        bind_token(&e, &extra);
    });
}

#[test]
fn bind_tokens_fits_current_bucket() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let tokens = e.as_contract(&address, || {
        // batch smaller than BUCKET_SIZE * 2
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..10u32 {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);

        // verify
        assert_eq!(linked_token_count(&e), 10);
        for i in 0..10u32 {
            assert_eq!(get_token_by_index(&e, i), batch.get(i).unwrap());
        }

        linked_tokens(&e)
    });

    assert_eq!(tokens.len(), 10);
}

#[test]
fn bind_tokens_splits_across_two_buckets() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Pre-fill current bucket to BUCKET_SIZE - 5
        for _ in 0..95u32 {
            let t = Address::generate(&e);
            bind_token(&e, &t);
        }
        assert_eq!(linked_token_count(&e), 95);

        // Now bind a batch of 10 which should split: 5 in current, 5 in next
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..10u32 {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);

        // Validate counts
        assert_eq!(linked_token_count(&e), 105);

        // First 5 go at indices 95..99 (current bucket), next 5 at 100..104 (next
        // bucket)
        for i in 0..10u32 {
            assert_eq!(get_token_by_index(&e, 95 + i), batch.get(i).unwrap());
        }

        // Spot-check full list end ordering
        let all = linked_tokens(&e);
        assert_eq!(all.len(), 105);
        for i in 0..10u32 {
            assert_eq!(all.get(95 + i).unwrap(), batch.get(i).unwrap());
        }
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
        assert_eq!(get_token_by_index(&e, 0), token);
        assert_eq!(get_token_index(&e, &token), 0);
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

        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token3);

        assert_eq!(get_token_index(&e, &token1), 0);
        assert_eq!(get_token_index(&e, &token3), 1);
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

        assert_eq!(get_token_by_index(&e, 0), token1);
        assert_eq!(get_token_by_index(&e, 1), token2);
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
#[should_panic(expected = "Error(Contract, #330)")]
fn get_token_by_invalid_index() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        get_token_by_index(&e, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")]
fn get_token_index_nonexistent() {
    let e = Env::default();
    let address = e.register(MockContract, ());
    let token = Address::generate(&e);

    e.as_contract(&address, || {
        get_token_index(&e, &token);
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
        assert_eq!(get_token_index(&e, &token), 0);

        unbind_token(&e, &token);
        assert!(!is_token_bound(&e, &token));

        bind_token(&e, &token);
        assert!(is_token_bound(&e, &token));
        assert_eq!(get_token_index(&e, &token), 0);
        assert_eq!(linked_token_count(&e), 1);
    });
}

#[test]
fn bind_tokens_spill_across_three_buckets() {
    let e = Env::default();
    e.cost_estimate().disable_resource_limits();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Pre-fill 50 tokens in current bucket
        for _ in 0..50u32 {
            bind_token(&e, &Address::generate(&e));
        }
        assert_eq!(linked_token_count(&e), 50);

        // Batch of 200 should fill: 50 in current, 100 in next, 50 in third
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..200u32 {
            batch.push_back(Address::generate(&e));
        }

        bind_tokens(&e, &batch);

        assert_eq!(linked_token_count(&e), 250);

        // First 50 of batch at indices 50..99
        for i in 0..50u32 {
            assert_eq!(get_token_by_index(&e, 50 + i), batch.get(i).unwrap());
        }
        // Next 100 of batch at indices 100..199
        for i in 0..100u32 {
            assert_eq!(get_token_by_index(&e, 100 + i), batch.get(50 + i).unwrap());
        }
        // Last 50 of batch at indices 200..249
        for i in 0..50u32 {
            assert_eq!(get_token_by_index(&e, 200 + i), batch.get(150 + i).unwrap());
        }
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #333)")]
fn bind_tokens_batch_too_large_should_panic() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        let target_len = BUCKET_SIZE * 2 + 1; // strictly greater than allowed
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
fn unbind_makes_last_bucket_empty() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Create exactly BUCKET_SIZE + 1 tokens => second bucket has 1 item
        let mut tokens: Vec<Address> = Vec::new(&e);
        for _ in 0..(BUCKET_SIZE + 1) {
            let t = Address::generate(&e);
            bind_token(&e, &t);
            tokens.push_back(t);
        }
        assert_eq!(linked_token_count(&e), BUCKET_SIZE + 1);

        // Unbind the first token; the single last token moves into index 0,
        // making the last bucket empty afterwards.
        let first = tokens.get(0).unwrap();
        let last_before = tokens.get(BUCKET_SIZE).unwrap();
        unbind_token(&e, &first);

        assert_eq!(linked_token_count(&e), BUCKET_SIZE);
        // Index 0 now holds the previous last token
        assert_eq!(get_token_by_index(&e, 0), last_before);

        // Sanity: full list has exactly BUCKET_SIZE elements now
        let all = linked_tokens(&e);
        assert_eq!(all.len(), BUCKET_SIZE);
    });
}

#[test]
fn is_token_bound_across_buckets() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    let (t0, t_boundary, t_next) = e.as_contract(&address, || {
        // Fill first bucket fully and add a few into next
        let mut first: Option<Address> = None;
        for i in 0..(BUCKET_SIZE + 5) {
            let t = Address::generate(&e);
            bind_token(&e, &t);
            if i == 0 {
                first = Some(t.clone());
            }
        }
        let first_token = first.unwrap();
        let boundary_token = get_token_by_index(&e, BUCKET_SIZE - 1);
        let next_bucket_token = get_token_by_index(&e, BUCKET_SIZE);
        (first_token, boundary_token, next_bucket_token)
    });

    e.as_contract(&address, || {
        assert!(is_token_bound(&e, &t0));
        assert!(is_token_bound(&e, &t_boundary));
        assert!(is_token_bound(&e, &t_next));
    });
}

#[test]
fn bind_tokens_exact_bucket_boundary_start() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        // Pre-fill exactly one bucket
        for _ in 0..BUCKET_SIZE {
            bind_token(&e, &Address::generate(&e));
        }
        assert_eq!(linked_token_count(&e), BUCKET_SIZE);

        // Now bind a batch that fits entirely in the next bucket
        let mut batch: Vec<Address> = Vec::new(&e);
        for _ in 0..10u32 {
            batch.push_back(Address::generate(&e));
        }
        bind_tokens(&e, &batch);

        assert_eq!(linked_token_count(&e), BUCKET_SIZE + 10);
        for i in 0..10u32 {
            assert_eq!(get_token_by_index(&e, BUCKET_SIZE + i), batch.get(i).unwrap());
        }
    });
}
