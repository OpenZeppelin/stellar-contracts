use soroban_sdk::{contracttype, panic_with_error, symbol_short, Address, Env};
use stellar_constants::{BALANCE_EXTEND_AMOUNT, BALANCE_TTL_THRESHOLD};

use crate::{
    fungible::FungibleTokenError,
    overrides::{Base, ContractOverrides},
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

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AllowListError {
    /// The user is not allowed to perform this operation
    UserNotAllowed = 300,
}

/// Storage keys for the data associated with the allowlist extension
#[contracttype]
pub enum AllowListStorageKey {
    /// Stores the allowed status of an account
    Allowed(Address),
    /// Stores the admin address
    Admin,
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
        e.storage().persistent().get(&key).unwrap_or(false)
    }

    /// Returns the admin address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn admin(e: &Env) -> Address {
        e.storage().instance().get(&AllowListStorageKey::Admin).unwrap()
    }

    // ################## CHANGE STATE ##################

    /// Sets the admin address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address to set as admin.
    pub fn set_admin(e: &Env, admin: &Address) {
        e.storage().instance().set(&AllowListStorageKey::Admin, admin);
    }

    /// Allows a user to receive and transfer tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to allow.
    ///
    /// # Errors
    ///
    /// * Panics if `admin` is not the admin.
    ///
    /// # Events
    ///
    /// * topics - `["user_allowed", user: Address]`
    /// * data - `[]`
    pub fn allow_user(e: &Env, admin: &Address, user: &Address) {
        // Verify admin authorization
        admin.require_auth();
        
        // Check if the caller is the admin
        let stored_admin = AllowList::admin(e);
        if *admin != stored_admin {
            panic!("only admin can allow users");
        }

        // Set the user as allowed
        let key = AllowListStorageKey::Allowed(user.clone());
        e.storage().persistent().set(&key, &true);
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);

        // Emit event
        let topics = (symbol_short!("user_allowed"), user);
        e.events().publish(topics, ());
    }

    /// Disallows a user from receiving and transferring tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `admin` - The address of the admin performing the operation.
    /// * `user` - The address to disallow.
    ///
    /// # Errors
    ///
    /// * Panics if `admin` is not the admin.
    ///
    /// # Events
    ///
    /// * topics - `["user_disallowed", user: Address]`
    /// * data - `[]`
    pub fn disallow_user(e: &Env, admin: &Address, user: &Address) {
        // Verify admin authorization
        admin.require_auth();
        
        // Check if the caller is the admin
        let stored_admin = AllowList::admin(e);
        if *admin != stored_admin {
            panic!("only admin can disallow users");
        }

        // Set the user as not allowed
        let key = AllowListStorageKey::Allowed(user.clone());
        e.storage().persistent().set(&key, &false);
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);

        // Emit event
        let topics = (symbol_short!("user_disallowed"), user);
        e.events().publish(topics, ());
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
    /// * [`AllowListError::UserNotAllowed`] - When either `from` or `to` is not allowed.
    /// * Also refer to [`Base::transfer`] errors.
    pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        // Check if both addresses are allowed
        if !AllowList::allowed(e, from) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }
        if !AllowList::allowed(e, to) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }

        // Call the base implementation
        Base::transfer(e, from, to, amount);
    }

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. `amount` is then deducted from `spender`s allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its allowance
    ///   consumed during the transfer.
    /// * `from` - The address holding the tokens which will be transferred.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`AllowListError::UserNotAllowed`] - When either `from`, `to`, or `spender` is not allowed.
    /// * Also refer to [`Base::transfer_from`] errors.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        // Check if all addresses are allowed
        if !AllowList::allowed(e, spender) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }
        if !AllowList::allowed(e, from) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }
        if !AllowList::allowed(e, to) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }

        // Call the base implementation
        Base::transfer_from(e, spender, from, to, amount);
    }

    /// Sets the amount of tokens a `spender` is allowed to spend on behalf of an
    /// `owner`. Overrides any existing allowance set between `spender` and `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    /// * `amount` - The amount of tokens made available to `spender`.
    /// * `live_until_ledger` - The ledger number at which the allowance expires.
    ///
    /// # Errors
    ///
    /// * [`AllowListError::UserNotAllowed`] - When either `owner` or `spender` is not allowed.
    /// * Also refer to [`Base::approve`] errors.
    pub fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        // Check if both addresses are allowed
        if !AllowList::allowed(e, owner) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }
        if !AllowList::allowed(e, spender) {
            panic_with_error!(e, AllowListError::UserNotAllowed);
        }

        // Call the base implementation
        Base::approve(e, owner, spender, amount, live_until_ledger);
    }
}
