use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_constants::{ALLOW_BLOCK_EXTEND_AMOUNT, ALLOW_BLOCK_TTL_THRESHOLD};

use crate::{
    extensions::allowlist::{emit_user_allowed, emit_user_disallowed},
    overrides::{Base, ContractOverrides},
    FungibleTokenError,
};

pub struct AllowList;

impl ContractOverrides for AllowList {
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

/// Storage keys for the data associated with the allowlist extension
#[contracttype]
pub enum AllowListStorageKey {
    /// Stores the allowed status of an account
    Allowed(Address),
}

impl AllowList {
    // ################## QUERY STATE ##################

    /// Returns the allowed status of an account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check the allowed status for.
    pub fn allowed(e: &Env, account: &Address) -> bool {
        let key = AllowListStorageKey::Allowed(account.clone());
        if let Some(allowed) = e.storage().persistent().get(&key) {
            e.storage().persistent().extend_ttl(
                &key,
                ALLOW_BLOCK_TTL_THRESHOLD,
                ALLOW_BLOCK_EXTEND_AMOUNT,
            );
            allowed
        } else {
            false
        }
    }

    // ################## CHANGE STATE ##################

    /// Allows a user to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to allow.
    ///
    /// # Events
    ///
    /// * topics - `["allow", user: Address]`
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
    /// security risks as it could allow unauthorized allowlist
    /// modifications.
    pub fn allow_user(e: &Env, user: &Address) {
        // Set the user as allowed
        let key = AllowListStorageKey::Allowed(user.clone());
        e.storage().persistent().set(&key, &true);

        // Emit event
        emit_user_allowed(e, user);
    }

    /// Disallows a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user` - The address to disallow.
    ///
    /// # Events
    ///
    /// * topics - `["disallow", user: Address]`
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
    /// security risks as it could allow unauthorized allowlist
    /// modifications.
    pub fn disallow_user(e: &Env, user: &Address) {
        // Set the user as not allowed
        let key = AllowListStorageKey::Allowed(user.clone());
        e.storage().persistent().set(&key, &false);

        // Emit event
        emit_user_disallowed(e, user);
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
    /// * [`FungibleTokenError::UserNotAllowed`] - When either `from` or `to` is
    ///   not allowed.
    /// * Also refer to [`Base::transfer`] errors.
    pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        // Check if both addresses are allowed
        if !AllowList::allowed(e, from) || !AllowList::allowed(e, to) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
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
    /// * [`FungibleTokenError::UserNotAllowed`] - When either `from`, or `to`
    ///   is not allowed.
    /// * Also refer to [`Base::transfer_from`] errors.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        // Check if all addresses are allowed
        if !AllowList::allowed(e, from) || !AllowList::allowed(e, to) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
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
    /// * [`FungibleTokenError::UserNotAllowed`] - When `owner` is not allowed.
    /// * Also refer to [`Base::approve`] errors.
    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        // Check if both addresses are allowed
        if !AllowList::allowed(e, owner) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        // Call the base implementation
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }
}
