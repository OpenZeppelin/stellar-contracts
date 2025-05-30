use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_constants::{BALANCE_EXTEND_AMOUNT, BALANCE_TTL_THRESHOLD};

use crate::{
    extensions::blocklist::{emit_user_blocked, emit_user_unblocked},
    overrides::{Base, ContractOverrides},
    FungibleTokenError,
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
    /// Stores the admin address
    Admin,
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
        e.storage().persistent().get(&key).unwrap_or(false)
    }

    /// Returns the admin address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn get_admin(e: &Env) -> Address {
        e.storage().instance().get(&BlockListStorageKey::Admin).unwrap()
    }

    // ################## CHANGE STATE ##################

    /// Sets the admin address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address to set as admin.
    ///
    /// **IMPORTANT**: this function lacks authorization checks.
    /// It is expected to call this function only in the constructor!
    pub fn set_admin(e: &Env, admin: &Address) {
        e.storage().instance().set(&BlockListStorageKey::Admin, admin);
    }

    /// Blocks a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    ///
    /// # Events
    ///
    /// * topics - `["user_blocked", user: Address]`
    /// * data - `[]`
    pub fn block_user(e: &Env, user: &Address) {
        // Verify admin authorization
        let admin = BlockList::get_admin(e);
        admin.require_auth();

        // Call the no_auth implementation
        BlockList::block_user_no_auth(e, user);
    }

    /// Low-level function to block a user without performing authorization
    /// checks.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to block.
    ///
    /// # Events
    ///
    /// * topics - `["user_blocked", user: Address]`
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
    pub fn block_user_no_auth(e: &Env, user: &Address) {
        // Set the user as blocked
        let key = BlockListStorageKey::Blocked(user.clone());
        e.storage().persistent().set(&key, &true);
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);

        // Emit event
        emit_user_blocked(e, user);
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
    /// * topics - `["user_unblocked", user: Address]`
    /// * data - `[]`
    pub fn unblock_user(e: &Env, user: &Address) {
        // Verify admin authorization
        let admin = BlockList::get_admin(e);
        admin.require_auth();

        // Call the no_auth implementation
        BlockList::unblock_user_no_auth(e, user);
    }

    /// Low-level function to unblock a user without performing authorization
    /// checks.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to unblock.
    ///
    /// # Events
    ///
    /// * topics - `["user_unblocked", user: Address]`
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
    pub fn unblock_user_no_auth(e: &Env, user: &Address) {
        // Set the user as not blocked
        let key = BlockListStorageKey::Blocked(user.clone());
        e.storage().persistent().set(&key, &false);
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);

        // Emit event
        emit_user_unblocked(e, user);
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
        // Check if either address is blocked
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        if BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        // Call the base implementation
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
    /// * [`FungibleTokenError::UserBlocked`] - When either `from`, `to`, or
    ///   `spender` is blocked.
    /// * Also refer to [`Base::transfer_from`] errors.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        // Check if any address is blocked
        if BlockList::blocked(e, spender) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        if BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        // Call the base implementation
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
    /// * [`FungibleTokenError::UserBlocked`] - When either `owner` or `spender`
    ///   is blocked.
    /// * Also refer to [`Base::approve`] errors.
    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        // Check if either address is blocked
        if BlockList::blocked(e, owner) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        if BlockList::blocked(e, spender) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        // Call the base implementation
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }
}
