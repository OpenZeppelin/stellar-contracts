use soroban_sdk::{contracttype, Address, Env};

use crate::{
    fungible::{
        extensions::blocklist::{emit_user_blocked, emit_user_unblocked, FungibleBlockList},
        FTBase, FungibleToken, ALLOW_BLOCK_EXTEND_AMOUNT, ALLOW_BLOCK_TTL_THRESHOLD,
    },
    FungibleBurnable,
};

pub struct BlockList;

impl FungibleBlockList for BlockList {
    type Impl = Self;

    /// Returns the blocked status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the blocked status for.
    fn blocked(e: &Env, account: &Address) -> bool {
        let key = BlockListStorageKey::Blocked(account.clone());
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

    /// Blocks a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    fn block_user(e: &Env, user: &Address, _operator: &Address) {
        let key = BlockListStorageKey::Blocked(user.clone());

        // if the user is not blocked, block them
        if !e.storage().persistent().has(&key) {
            e.storage().persistent().set(&key, &());

            emit_user_blocked(e, user);
        }
    }

    /// Unblocks a user, allowing them to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to unblock.
    fn unblock_user(e: &Env, user: &Address, _operator: &Address) {
        let key = BlockListStorageKey::Blocked(user.clone());

        // if the user is currently blocked, unblock them
        if e.storage().persistent().has(&key) {
            e.storage().persistent().remove(&key);
            emit_user_unblocked(e, user);
        }
    }
}

impl FungibleToken for BlockList {
    type Impl = FTBase;

    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Self::assert_not_blocked(e, &[owner]);
        Self::Impl::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl FungibleBurnable for BlockList {
    type Impl = FTBase;

    fn burn(e: &Env, from: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from]);
        Self::Impl::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from]);
        Self::Impl::burn_from(e, spender, from, amount);
    }
}

/// Storage keys for the data associated with the blocklist extension
#[contracttype]
pub enum BlockListStorageKey {
    /// Stores the blocked status of an account
    Blocked(Address),
}
