use soroban_sdk::{contracttype, panic_with_error, Address, Env};

#[cfg(not(feature = "certora"))]
use crate::fungible::{
    extensions::blocklist::{emit_user_blocked, emit_user_unblocked},
};

use crate::fungible::{
    overrides::{Base, ContractOverrides},
    FungibleTokenError, ALLOW_BLOCK_EXTEND_AMOUNT, ALLOW_BLOCK_TTL_THRESHOLD,
};

pub struct BlockList;

impl ContractOverrides for BlockList {
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        BlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

/// Storage keys for the data associated with the blocklist extension
#[contracttype]
pub enum BlockListStorageKey {
    /// Stores the blocked status of an account
    Blocked(Address),
}

impl BlockList {
    // ################## QUERY STATE ##################

    /// Returns the blocked status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the blocked status for.
    pub fn blocked(e: &Env, account: &Address) -> bool {
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

    // ################## CHANGE STATE ##################

    /// Blocks a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    ///
    /// # Events
    ///
    /// * topics - `["block", user: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used:
    /// - During contract initialization/construction
    /// - In admin functions that implement their own authorization logic
    ///
    /// Using this function in public-facing methods creates significant
    /// security risks as it could allow unauthorized blocklist
    /// modifications.
    pub fn block_user(e: &Env, user: &Address) {
        let key = BlockListStorageKey::Blocked(user.clone());

        // if the user is not blocked, block them
        if !e.storage().persistent().has(&key) {
            e.storage().persistent().set(&key, &());
            #[cfg(not(feature = "certora"))]
            emit_user_blocked(e, user);
        }
    }

    /// Unblocks a user, allowing them to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to unblock.
    ///
    /// # Events
    ///
    /// * topics - `["unblock", user: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used:
    /// - During contract initialization/construction
    /// - In admin functions that implement their own authorization logic
    ///
    /// Using this function in public-facing methods creates significant
    /// security risks as it could allow unauthorized blocklist
    /// modifications.
    pub fn unblock_user(e: &Env, user: &Address) {
        let key = BlockListStorageKey::Blocked(user.clone());

        // if the user is currently blocked, unblock them
        if e.storage().persistent().has(&key) {
            e.storage().persistent().remove(&key);
            #[cfg(not(feature = "certora"))]
            emit_user_unblocked(e, user);
        }
    }

    // ################## OVERRIDDEN FUNCTIONS ##################

    /// Transfers `amount` of tokens from `from` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When either `from` or `to` is
    ///   blocked.
    /// * Also refer to [`Base::transfer`] errors.
    pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        Base::transfer(e, from, to, amount);
    }

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. `amount` is then deducted from `spender`s
    /// allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance consumed during the transfer.
    /// * `from` - The address holding the tokens which will be transferred.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When either `from`, or `to` is
    ///   blocked.
    /// * Also refer to [`Base::transfer_from`] errors.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        Base::transfer_from(e, spender, from, to, amount);
    }

    /// Sets the amount of tokens a `spender` is allowed to spend on behalf of
    /// an `owner`. Overrides any existing allowance set between `spender`
    /// and `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    /// * `amount` - The amount of tokens made available to `spender`.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When `owner` is blocked.
    /// * Also refer to [`Base::approve`] errors.
    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        if BlockList::blocked(e, owner) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        Base::approve(e, owner, spender, amount, live_until_ledger);
    }

    /// This is a wrapper around [`Base::burn()`] to enable
    /// the compatibility across [`crate::fungible::burnable::FungibleBurnable`]
    /// with [`crate::fungible::blocklist::FungibleBlockList`]
    ///
    /// Please refer to [`Base::burn`] for the inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        Base::burn(e, from, amount);
    }

    /// This is a wrapper around [`Base::burn_from()`] to enable
    /// the compatibility across [`crate::fungible::burnable::FungibleBurnable`]
    /// with [`crate::fungible::blocklist::FungibleBlockList`]
    ///
    /// Please refer to [`Base::burn_from`] for the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        Base::burn_from(e, spender, from, amount);
    }
}
