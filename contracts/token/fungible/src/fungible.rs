use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Env, String};

/// Vanilla Fungible Token Trait
///
/// The `FungibleToken` trait defines the core functionality for fungible
/// tokens, adhering to SEP-41. It provides a standard interface for managing
/// balances, allowances, and metadata associated with fungible tokens.
/// Additionally, this trait includes the `total_supply()` function, which is
/// not part of SEP-41 but is commonly used in token contracts.
///
/// To fully comply with the SEP-41 specification one have to implement the
/// `Burnable` trait in addition to this one. SEP-41 mandates support for token
/// burning to be considered compliant.
#[contractclient(name = "FungibleTokenClient")]
pub trait FungibleToken {
    /// Returns the total amount of tokens in circulation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::total_supply()`] function from
    /// the `storage` module when implementing this function.
    fn total_supply(e: &Env) -> i128;

    /// Returns the amount of tokens held by `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address for which the balance is being queried.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::storage::balance()`] function from
    /// the `storage` module when implementing this function.
    fn balance(e: &Env, account: Address) -> i128;

    /// Returns the amount of tokens a `spender` is allowed to spend on behalf
    /// of an `owner`.
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
    fn allowance(e: &Env, owner: Address, spender: Address) -> i128;

    /// Transfers a `value` amount of tokens from `from` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the transferred tokens.
    /// * `value` - The value of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
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
    fn transfer(e: &Env, from: Address, to: Address, value: i128);

    /// Transfers a `value` amount of tokens from `from` to `to` using the
    /// allowance mechanism. `value` is then deducted from `spender`
    /// allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance consumed during the transfer.
    /// * `from` - The address holding the tokens which will be transferred.
    /// * `to` - The address receiving the transferred tokens.
    /// * `value` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
    ///   transfer more tokens than `spender` current allowance.
    ///
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
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, value: i128);

    /// Sets the amount of tokens a `spender` is allowed to spend on behalf of
    /// an `owner`. Overrides any existing allowance set between `spender` and
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    /// * `value` - The amount of tokens made available to `spender`.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when
    ///   attempting to set `live_until_ledger` that is less than the current
    ///   ledger number and greater than `0`.
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
    fn approve(e: &Env, owner: Address, spender: Address, value: i128, live_until_ledger: u32);

    /// Returns the number of decimals used to represent amounts of this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::metadata::decimals()`] function from
    /// the `metadata` module when implementing this function.
    fn decimals(e: &Env) -> u32;

    /// Returns the name for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::metadata::name()`] function from
    /// the `metadata` module when implementing this function.
    fn name(e: &Env) -> String;

    /// Returns the symbol for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    ///
    /// # Notes
    ///
    /// We recommend using the [`crate::metadata::symbol()`] function from
    /// the `metadata` module when implementing this function.
    fn symbol(e: &Env) -> String;
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

/// Emits an event indicating a transfer of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `value` - The value of tokens to be transferred.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[value: i128]`
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, value: i128) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, value)
}

/// Emits an event indicating an allowance was set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `value` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///
/// # Events
///
/// * topics - `["approve", owner: Address, spender: Address]`
/// * data - `[value: i128, live_until_ledger: u32]`
pub fn emit_approve(
    e: &Env,
    owner: &Address,
    spender: &Address,
    value: i128,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approve"), owner, spender);
    e.events().publish(topics, (value, live_until_ledger))
}
