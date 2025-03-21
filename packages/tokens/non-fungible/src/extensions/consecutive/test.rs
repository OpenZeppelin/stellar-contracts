#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, testutils::Address as _, Address, Env,
};

use crate::{
    balance,
    consecutive::storage::{batch_mint, burn, owner_of, transfer, StorageKey},
    NonFungibleTokenError,
};

use super::{IBurnable, IMintable, INonFungibleBase, INonFungibleConsecutive, ISequential};

#[contract]
pub struct MockContract;

impl INonFungibleBase for MockContract {
    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        transfer::<Self>(e, &from, &to, token_id);
    }

    fn owner_of(e: &Env, token_id: u32) -> Address {
        owner_of::<Self>(e, token_id)
    }

    fn increase_balance(e: &Env, to: Address, amount: u32) {
        //crate::storage::increase_balance::<Self>()

        let Some(balance) = balance(e, &to).checked_add(amount) else {
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().persistent().set(&StorageKey::Balance(to.clone()), &balance);
    }

    fn decrease_balance(e: &Env, from: Address, amount: u32) {
        //crate::storage::decrease_balance::<Self>()

        let Some(balance) = balance(e, &from).checked_sub(amount) else {
            // TODO: underflow ??
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().persistent().set(&StorageKey::Balance(from.clone()), &balance);
    }
}

#[contractimpl]
impl IMintable for MockContract {
    fn mint(e: &Env, to: Address, token_id: u32) -> u32 {
        // custom mint or crate::mintable::mint()
        // check Self::owner_of(e, token_id)
        crate::storage::update(e, None, Some(&to), token_id);
        crate::mintable::emit_mint(e, &to, token_id);
        token_id
    }
}

impl IBurnable for MockContract {
    fn burn(e: &Env, from: Address, token_id: u32) {
        unimplemented!()
    }

    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        unimplemented!()
    }
}

//#[contractimpl]
impl ISequential for MockContract {
    fn next_token_id(e: &Env) -> u32 {
        //crate::sequential::next_token_id::<Self>()
        e.storage().instance().get(&StorageKey::TokenIdCounter).unwrap_or(0)
    }

    fn increment_token_id(e: &Env) -> u32 {
        Self::increment_token_id_by(e, 1)
    }

    fn increment_token_id_by(e: &Env, amount: u32) -> u32 {
        //crate::sequential::increment_token_id_by::<Self>()
        let current_id = Self::next_token_id(e);
        let Some(next_id) = current_id.checked_add(amount) else {
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().instance().set(&StorageKey::TokenIdCounter, &next_id);
        current_id
    }
}

impl INonFungibleConsecutive for MockContract {
    fn batch_mint(e: &Env, to: Address, amount: u32) {
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

        assert_eq!(<MockContract as ISequential>::next_token_id(&e), amount);
        assert_eq!(balance(&e, &owner), amount);

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
        assert_eq!(balance(&e, &owner), amount);

        transfer::<MockContract>(&e, &owner, &recipient, 50);
        assert_eq!(owner_of::<MockContract>(&e, 50), recipient);
        assert_eq!(balance(&e, &recipient), 1);

        assert_eq!(owner_of::<MockContract>(&e, 51), owner);
        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(51)).unwrap();
        assert_eq!(_owner, owner);
    });

    e.as_contract(&address, || {
        transfer::<MockContract>(&e, &owner, &recipient, 99);
        assert_eq!(owner_of::<MockContract>(&e, 99), recipient);
        assert_eq!(balance(&e, &recipient), 2);
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
        assert_eq!(balance(&e, &owner), amount);

        burn::<MockContract>(&e, &owner, token_id);
        assert_eq!(balance(&e, &owner), amount - 1);

        let _owner = e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id));
        assert_eq!(_owner, None);
        let _owner =
            e.storage().persistent().get::<_, Address>(&StorageKey::Owner(token_id + 1)).unwrap();
        assert_eq!(_owner, owner);
    });
}
