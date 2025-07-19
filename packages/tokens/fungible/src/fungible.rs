use soroban_sdk::{contracterror, symbol_short, Address, Env};

/// Vanilla Fungible Token Trait
///
/// The `FungibleToken` trait defines the core functionality for fungible
/// tokens, adhering to SEP-41. It provides a standard interface for managing
/// balances, allowances, and metadata associated with fungible tokens.
/// Additionally, this trait includes the `total_supply()` function, which is
/// not part of SEP-41 but is commonly used in token contracts.
///
/// To fully comply with the SEP-41 specification one has to implement the
/// `FungibleBurnable` trait in addition to this one. SEP-41 mandates support
/// for token burning to be considered compliant.
///
/// Event for `mint` is defined, but `mint` function itself is not included
/// as a method in this trait because it is not a part of the SEP-41 standard,
/// the function signature may change depending on the implementation.
///
/// We do provide a function [`crate::Base::mint`] for minting to cover the
/// general use case.
///
/// # Notes
///
/// `#[contractimpl]` macro requires even the default implementations to be
/// present under its scope. To not confuse the developers, we did not provide
/// the default implementations here, but we are providing a macro to generate
/// them.
///
/// When implementing [`NonFungibleToken`] trait for your Smart Contract,
/// you can follow the below example:
///
/// ```ignore
/// #[default_impl] // **IMPORTANT**: place this above `#[contractimpl]`
/// #[contractimpl]
/// impl FungibleToken for MyContract {
///     ContractType = {Your Contract Type Here};
///
///     /* your overrides here (you don't have to put anything here if you don't want to override anything) */
///     /* and the macro will generate all the missing default implementations for you */
/// }
/// ```
///
/// This trait is implemented for the following Contract Types:
/// * [`crate::Base`] (covering the vanilla case, and compatible with
///   [`crate::extensions::burnable::FungibleBurnable`]) trait
/// * [`crate::extensions::allowlist::AllowList`] (enabling the compatibility
///   and overrides for [`crate::extensions::allowlist::FungibleAllowList`])
///   trait, incompatible with [`crate::extensions::blocklist::BlockList`]
///   trait.
/// * [`crate::extensions::blocklist::BlockList`] (enabling the compatibility
///   and overrides for [`crate::extensions::blocklist::FungibleBlockList`])
///   trait, incompatible with [`crate::extensions::allowlist::AllowList`]
///   trait.
///
/// You can find the default implementations of this trait for `Base`,
/// `Allowlist`, and `Blocklist` by navigating to:
/// `ContractType::{method_name}`. For example, if you want to find how
/// [`FungibleToken::transfer`] is implemented for the `Allowlist` contract
/// type, you can find it using
/// [`crate::extensions::allowlist::AllowList::transfer`].
#[soroban_sdk::contracttrait(default = Base)]
pub trait FungibleToken {
    /// Helper type that allows us to override some of the functionality of the
    /// base trait based on the extensions implemented. You should use
    /// [`crate::Base`] as the type if you are not using
    /// [`crate::extensions::allowlist::AllowList`] or
    /// [`crate::extensions::blocklist::BlockList`] extensions.
    // type ContractType: ContractOverrides;
    /// Returns the total amount of tokens in circulation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> i128;

    /// Returns the amount of tokens held by `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address for which the balance is being queried.
    fn balance(e: &Env, account: &soroban_sdk::Address) -> i128;

    /// Returns the amount of tokens a `spender` is allowed to spend on behalf
    /// of an `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `spender` - The address authorized to spend the tokens.
    fn allowance(e: &Env, owner: &soroban_sdk::Address, spender: &soroban_sdk::Address) -> i128;

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
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn transfer(e: &Env, from: &soroban_sdk::Address, to: &soroban_sdk::Address, amount: i128);

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. `amount` is then deducted from `spender`
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
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    /// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
    ///   transfer more tokens than `spender` current allowance.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn transfer_from(
        e: &Env,
        spender: &soroban_sdk::Address,
        from: &soroban_sdk::Address,
        to: &soroban_sdk::Address,
        amount: i128,
    );

    /// Sets the amount of tokens a `spender` is allowed to spend on behalf of
    /// an `owner`. Overrides any existing allowance set between `spender` and
    /// `owner`.
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
    /// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when
    ///   attempting to set `live_until_ledger` that is less than the current
    ///   ledger number and greater than `0`.
    /// * [`FungibleTokenError::LessThanZero`] - Occurs when `amount < 0`.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, spender: Address]`
    /// * data - `[amount: i128, live_until_ledger: u32]`
    fn approve(
        e: &Env,
        owner: &soroban_sdk::Address,
        spender: &soroban_sdk::Address,
        amount: i128,
        live_until_ledger: u32,
    );

    /// Returns the number of decimals used to represent amounts of this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn decimals(e: &Env) -> u32;

    /// Returns the name for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn name(e: &Env) -> soroban_sdk::String;

    /// Returns the symbol for this token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    fn symbol(e: &Env) -> soroban_sdk::String;

    // ### Internal Methods ###

    /// Creates `amount` of tokens and assigns them to `to`. Updates
    /// the total supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Errors
    ///
    /// refer to [`update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// minting operations. Failure to implement proper authorization could
    /// lead to security vulnerabilities and unauthorized token creation.
    ///
    /// You probably want to do something like this (pseudo-code):
    ///
    /// ```ignore
    /// let admin = read_administrator(e);
    /// admin.require_auth();
    /// ```
    #[internal]
    fn internal_mint(e: &Env, to: &soroban_sdk::Address, amount: i128);

    /// Sets the token metadata such as decimals, name and symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `decimals` - The number of decimals.
    /// * `name` - The name of the token.
    /// * `symbol` - The symbol of the token.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. You want to
    /// invoke it most likely from a constructor or from another function with
    /// admin-only authorization.
    #[internal]
    fn set_metadata(e: &Env, decimals: u32, name: soroban_sdk::String, symbol: soroban_sdk::String);
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FungibleTokenError {
    /// Indicates an error related to the current balance of account from which
    /// tokens are expected to be transferred.
    InsufficientBalance = 100,
    /// Indicates a failure with the allowance mechanism when a given spender
    /// doesn't have enough allowance.
    InsufficientAllowance = 101,
    /// Indicates an invalid value for `live_until_ledger` when setting an
    /// allowance.
    InvalidLiveUntilLedger = 102,
    /// Indicates an error when an input that must be >= 0
    LessThanZero = 103,
    /// Indicates overflow when adding two values
    MathOverflow = 104,
    /// Indicates access to uninitialized metadata
    UnsetMetadata = 105,
    /// Indicates that the operation would have caused `total_supply` to exceed
    /// the `cap`.
    ExceededCap = 106,
    /// Indicates the supplied `cap` is not a valid cap value.
    InvalidCap = 107,
    /// Indicates the Cap was not set.
    CapNotSet = 108,
    /// Indicates the SAC address was not set.
    SACNotSet = 109,
    /// Indicates a SAC address different than expected.
    SACAddressMismatch = 110,
    /// Indicates a missing function parameter in the SAC contract context.
    SACMissingFnParam = 111,
    /// Indicates an invalid function parameter in the SAC contract context.
    SACInvalidFnParam = 112,
    /// The user is not allowed to perform this operation
    UserNotAllowed = 113,
    /// The user is blocked and cannot perform this operation
    UserBlocked = 114,
}

// ################## EVENTS ##################

/// Emits an event indicating a transfer of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, amount)
}

/// Emits an event indicating an allowance was set.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///
/// # Events
///
/// * topics - `["approve", owner: Address, spender: Address]`
/// * data - `[amount: i128, live_until_ledger: u32]`
pub fn emit_approve(
    e: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approve"), owner, spender);
    e.events().publish(topics, (amount, live_until_ledger))
}

/// Emits an event indicating a mint of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `amount` - The amount of tokens to mint.
///
/// # Events
///
/// * topics - `["mint", account: Address]`
/// * data - `[amount: i128]`
pub fn emit_mint(e: &Env, to: &Address, amount: i128) {
    let topics = (symbol_short!("mint"), to);
    e.events().publish(topics, amount)
}
