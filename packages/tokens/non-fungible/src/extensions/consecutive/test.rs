#![allow(unused_variables)]
#![cfg(test)]

extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};

use crate::{
    consecutive::storage::{batch_mint, burn, owner_of, transfer, transfer_from, StorageKey},
    storage2::{approve, balance},
    NonFungibleToken,
};

use super::{IBurnable, IMintable, NonFungibleConsecutive, NonFungibleSequential};

#[contract]
pub struct MockContract;

#[contractimpl]
impl NonFungibleToken for MockContract {
    fn balance(e: &Env, owner: Address) -> u32 {
        crate::storage2::balance::<Self>(e, &owner)
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        crate::consecutive::storage::owner_of::<Self>(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        unimplemented!()
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u32) {
        unimplemented!()
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        unimplemented!()
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        unimplemented!()
    }

    fn get_approved(e: &Env, token_id: u32) -> Option<Address> {
        crate::storage2::get_approved::<Self>(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        crate::storage2::is_approved_for_all::<Self>(e, &owner, &operator)
    }

    fn name(e: &Env) -> String {
        unimplemented!()
    }

    fn symbol(e: &Env) -> String {
        unimplemented!()
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        unimplemented!()
    }
}

#[contractimpl]
impl IMintable for MockContract {
    fn mint(e: &Env, to: Address, token_id: u32) -> u32 {
        unimplemented!()
    }
}

#[contractimpl]
impl IBurnable for MockContract {
    fn burn(e: &Env, from: Address, token_id: u32) {
        unimplemented!()
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        unimplemented!()
    }
}

impl NonFungibleSequential for MockContract {
    fn next_token_id(e: &Env) -> u32 {
        crate::sequential::next_token_id::<Self>(e)
    }

    fn increment_token_id(e: &Env, amount: u32) -> u32 {
        crate::sequential::increment_token_id::<Self>(e, amount)
    }
}

impl NonFungibleConsecutive for MockContract {
    fn batch_mint(e: &Env, to: Address, amount: u32) {
        // access control
        batch_mint::<Self>(e, &to, amount);
    }
}

#[test]
fn consecutive_owner_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let amount = 100u32;

    e.as_contract(&address, || {
        batch_mint::<MockContract>(&e, &owner, amount);

        assert_eq!(MockContract::next_token_id(&e), amount);
        assert_eq!(balance::<MockContract>(&e, &owner), amount);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(0)).unwrap();
        assert_eq!(_owner, owner);
        assert_eq!(owner_of::<MockContract>(&e, 50), owner);
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
        batch_mint::<MockContract>(&e, &owner, amount);
        assert_eq!(balance::<MockContract>(&e, &owner), amount);

        transfer::<MockContract>(&e, &owner, &recipient, 50);
        assert_eq!(owner_of::<MockContract>(&e, 50), recipient);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);

        assert_eq!(owner_of::<MockContract>(&e, 51), owner);
        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(51)).unwrap();
        assert_eq!(_owner, owner);
    });

    e.as_contract(&address, || {
        transfer::<MockContract>(&e, &owner, &recipient, 99);
        assert_eq!(owner_of::<MockContract>(&e, 99), recipient);
        assert_eq!(balance::<MockContract>(&e, &recipient), 2);
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
        batch_mint::<MockContract>(&e, &owner, amount);
        assert_eq!(balance::<MockContract>(&e, &owner), amount);

        approve::<MockContract>(&e, &owner, &spender, token_id, 100);
        transfer_from::<MockContract>(&e, &spender, &owner, &recipient, token_id);
        assert_eq!(owner_of::<MockContract>(&e, token_id), recipient);
        assert_eq!(balance::<MockContract>(&e, &recipient), 1);

        assert_eq!(owner_of::<MockContract>(&e, token_id + 1), owner);
    });
}

#[test]
fn consecutive_burn_works() {
    let e = Env::default();
    e.mock_all_auths();
    let address = e.register(MockContract, ());

    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let amount = 100u32;
    let token_id = 50;

    e.as_contract(&address, || {
        batch_mint::<MockContract>(&e, &owner, amount);
        assert_eq!(balance::<MockContract>(&e, &owner), amount);

        burn::<MockContract>(&e, &owner, token_id);
        assert_eq!(balance::<MockContract>(&e, &owner), amount - 1);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner =
            e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id + 1)).unwrap();
        assert_eq!(_owner, owner);
    });
}
