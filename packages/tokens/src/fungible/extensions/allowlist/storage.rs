use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    fungible::{
        extensions::{
            allowlist::{emit_user_allowed, emit_user_disallowed},
            burnable::FungibleBurnable,
        },
        FungibleToken, ALLOW_BLOCK_EXTEND_AMOUNT, ALLOW_BLOCK_TTL_THRESHOLD,
    },
    FungibleTokenError,
};

pub struct AllowList;

/// Storage keys for the data associated with the allowlist extension
#[contracttype]
pub enum AllowListStorageKey {
    /// Stores the allowed status of an account
    Allowed(Address),
}

impl super::FungibleAllowList for AllowList {
    type Impl = Self;

    fn allowed(e: &Env, account: &Address) -> bool {
        let key = AllowListStorageKey::Allowed(account.clone());
        if e.storage().persistent().has(&key) {
            e.storage().persistent().extend_ttl(
                &key,
                ALLOW_BLOCK_TTL_THRESHOLD,
                ALLOW_BLOCK_EXTEND_AMOUNT,
            );
            true
        } else {
            false
        }
    }

    fn allow_user(e: &Env, user: &Address, _operator: &Address) {
        let key = AllowListStorageKey::Allowed(user.clone());

        // if the user is not allowed, allow them
        if !e.storage().persistent().has(&key) {
            e.storage().persistent().set(&key, &());

            emit_user_allowed(e, user);
        }
    }

    fn disallow_user(e: &Env, user: &Address, _operator: &Address) {
        let key = AllowListStorageKey::Allowed(user.clone());

        // if the user is currently allowed, disallow them
        if e.storage().persistent().has(&key) {
            e.storage().persistent().remove(&key);

            emit_user_disallowed(e, user);
        }
    }
}

impl<T: super::FungibleAllowList, N: FungibleToken> crate::fungible::FungibleToken
    for super::FungibleAllowListExt<T, N>
{
    type Impl = N;

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        if !T::allowed(e, from) || !T::allowed(e, to) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        N::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        if !T::allowed(e, from) || !T::allowed(e, to) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        N::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        if !T::allowed(e, owner) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        N::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl<T: super::FungibleAllowList, N: FungibleBurnable> FungibleBurnable
    for super::FungibleAllowListExt<T, N>
{
    type Impl = N;

    fn burn(e: &Env, from: &Address, amount: i128) {
        if !T::allowed(e, from) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        N::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        if !T::allowed(e, from) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }
        N::burn_from(e, spender, from, amount);
    }
}
