#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, testutils::Address as _, Address, Env};

use crate::{
    consecutive::storage::{
        approve, batch_mint, burn, owner_of, transfer, transfer_from, StorageKey,
    },
    sequential::next_token_id,
    storage::balance,
};

#[contract]
pub struct MockContract;

#[test]
fn consecutive_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100u32;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);

        assert_eq!(next_token_id(&e), amount);
        assert_eq!(balance(&e, &owner), amount);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0)).unwrap();
        assert_eq!(_owner, owner);
        assert_eq!(owner_of(&e, 50), owner);
    });
}

#[test]
fn consecutive_transfer_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100u32;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        transfer(&e, &owner, &recipient, 50);
        assert_eq!(owner_of(&e, 50), recipient);
        assert_eq!(balance(&e, &recipient), 1);

        assert_eq!(owner_of(&e, 51), owner);
        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(51)).unwrap();
        assert_eq!(_owner, owner);
    });

    e.as_contract(&address, || {
        transfer(&e, &owner, &recipient, 99);
        assert_eq!(owner_of(&e, 99), recipient);
        assert_eq!(balance(&e, &recipient), 2);
    });
}

#[test]
fn consecutive_transfer_from_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let spender = Address::generate(&e);
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100u32;
    let token_id = 50;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        approve(&e, &owner, &spender, token_id, 100);
        transfer_from(&e, &spender, &owner, &recipient, token_id);
        assert_eq!(owner_of(&e, token_id), recipient);
        assert_eq!(balance(&e, &recipient), 1);

        assert_eq!(owner_of(&e, token_id + 1), owner);
    });
}

#[test]
fn consecutive_burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100u32;
    let token_id = 50;

    e.as_contract(&address, || {
        batch_mint(&e, &owner, amount);
        assert_eq!(balance(&e, &owner), amount);

        burn(&e, &owner, token_id);
        assert_eq!(balance(&e, &owner), amount - 1);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner =
            e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id + 1)).unwrap();
        assert_eq!(_owner, owner);
    });
}
