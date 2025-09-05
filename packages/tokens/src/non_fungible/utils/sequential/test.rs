extern crate std;

use soroban_sdk::{contract, Env};

use crate::non_fungible::sequential::storage::{
    increment_token_id, next_token_id, NFTSequentialStorageKey,
};

#[contract]
pub struct MockContract;

#[test]
fn sequential_token_id_counter_increments() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        assert_eq!(next_token_id(&e), 0);

        let id1 = increment_token_id(&e, 10);
        assert_eq!(id1, 0);
        assert_eq!(next_token_id(&e), 10);

        let id2 = increment_token_id(&e, 5);
        assert_eq!(id2, 10);
        assert_eq!(next_token_id(&e), 15);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")]
fn sequential_increment_token_id_fails_on_overflow() {
    let e = Env::default();
    let address = e.register(MockContract, ());

    e.as_contract(&address, || {
        e.storage().instance().set(&NFTSequentialStorageKey::TokenIdCounter, &u32::MAX);
        let _ = increment_token_id(&e, 1);
    });
}
