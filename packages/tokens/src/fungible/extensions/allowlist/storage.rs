use soroban_sdk::{contracttype, Address, Env};

use crate::{
    fungible::{
        extensions::allowlist::{emit_user_allowed, emit_user_disallowed},
        ALLOW_BLOCK_EXTEND_AMOUNT, ALLOW_BLOCK_TTL_THRESHOLD,
    },
    FungibleAllowList,
};

pub struct AllowList;

/// Storage keys for the data associated with the allowlist extension
#[contracttype]
pub enum AllowListStorageKey {
    /// Stores the allowed status of an account
    Allowed(Address),
}

impl FungibleAllowList for AllowList {
    type Impl = Self;

    fn allowed(e: &Env, user: &Address) -> bool {
        let key = AllowListStorageKey::Allowed(user.clone());
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
    fn allow_user_no_auth(e: &Env, user: &Address) {
        let key = AllowListStorageKey::Allowed(user.clone());

        // if the user is not allowed, allow them
        if !e.storage().persistent().has(&key) {
            e.storage().persistent().set(&key, &());

            emit_user_allowed(e, user);
        }
    }

    fn allow_user(e: &Env, user: &Address, _operator: &Address) {
        Self::allow_user_no_auth(e, user);
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
