use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Env};

#[contractclient(name = "FungibleTokenClient")]
pub trait FungibleToken {
    /// Returns the number of tokens a `spender` is allowed to spend on behalf
    /// of an `owner` through [`transfer_from()`]. Defaults to `0`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::allowance()`] function from
    /// the `storage` module when implementing this function.
    fn allowance(e: Env, owner: Address, spender: Address) -> i128;

    /// Sets the number of tokens a `spender` is allowed to spend on behalf of
    /// an `owner`. Overrides any existing allowance set between `spender` and
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    /// * `value` - The number of tokens made available to `spender`.
    /// * `live_until_ledger` - The ledger number at which the allowance will
    ///   expire.
    ///
    /// # Errors
    ///
    /// When trying to set a value for `live_until_ledger` that is lower than
    /// the current ledger number and greater than `0`, then the error
    /// [`FungibleTokenError::InvalidLiveUntilLedger`] is thrown.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, spender: Address]`
    /// * data - `[value: i128, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::approve()`] function from
    /// the `storage` module when implementing this function.
    fn approve(e: Env, owner: Address, spender: Address, value: i128, live_until_ledger: u32);

    /// Returns the number of tokens held by an `account`. Defaults to `0` if
    /// no balance is stored.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `account` - The address for which a balance is being queried.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::balance()`] function from
    /// the `storage` module when implementing this function.
    fn balance(e: Env, account: Address) -> i128;

    /// Transfers a `value` number of tokens from `owner` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `to` - The address which will receive the transferred tokens.
    /// * `value` - The value of tokens to be transferred.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[value: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::transfer()`] function from
    /// the `storage` module when implementing this function.
    fn transfer(e: Env, owner: Address, to: Address, value: i128);

    /// Transfers a `value` number of tokens from `owner` to `to` using the
    /// allowance mechanism. `value` is then deducted from the `spender`
    /// allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance consumed during the transfer.
    /// * `owner` - The address holding the tokens which will be transferred.
    /// * `to` - The address which will receive the transferred tokens.
    /// * `value` - The number of tokens to be transferred.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[value: i128]`
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::transfer_from()`] function from
    /// the `storage` module when implementing this function.
    fn transfer_from(e: Env, spender: Address, owner: Address, to: Address, value: i128);
}

// ################## ERRORS ##################

#[contracterror]
#[repr(u32)]
pub enum FungibleTokenError {
    /// Indicates an error related to the current balance of account from which
    /// tokens are expected to be transferred.
    InsufficientBalance = 1,
    /// Indicates a failure with the allowance mechanism when a given spender
    /// doesn't have enough allowance.
    InsufficientAllowance = 2,
    /// Indicates an invalid value for `live_until_ledger` when setting an
    /// allowance.
    InvalidLiveUntilLedger = 3,
}

// ################## EVENTS ##################

pub fn emit_transfer(e: &Env, from: &Address, to: &Address, value: i128) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, value)
}

pub fn emit_approve(
    e: &Env,
    from: &Address,
    spender: &Address,
    value: i128,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approve"), from, spender);
    e.events().publish(topics, (value, live_until_ledger))
}
