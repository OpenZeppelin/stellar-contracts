use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Env, String};

/// Vanilla NonFungible Token Trait
///
/// The `NonFungibleToken` trait defines the core functionality for non-fungible
/// tokens. It provides a standard interface for managing
/// transfers and approvals associated with non-fungible tokens.
#[contractclient(name = "NonFungibleTokenClient")]
pub trait NonFungibleToken {
    /// Returns the number of tokens in `owner`'s account.
    ///
    /// # Arguments
    ///
    /// * `owner` - Account of the token's owner.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidOwner`] - If owner address is
    ///   `Address::ZERO`.
    fn balance_of(e: &Env, owner: Address) -> U256;

    /// Returns the owner of the `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    fn owner_of(e: &Env, token_id: U256) -> Address;

    /// Safely transfers `token_id` token from `from` to `to`, checking first
    /// that contract recipients are aware of the [`Erc721`] protocol to
    /// prevent tokens from being forever locked.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::IncorrectOwner`]  - If the previous owner is
    ///   not `from`.
    /// * [`NonFungibleTokenError::InsufficientApproval`] - If the caller does
    ///   not have the right to approve.
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    /// * [`NonFungibleTokenError::InvalidReceiver`] - If
    ///   [`IERC721Receiver::on_erc_721_received`] hasn't returned its
    /// interface id or returned with error, `to` is `Address::ZERO`.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: i128]`
    fn safe_transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: U256);

    /// Safely transfers `token_id` token from `from` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    /// * `data` - Additional data with no specified format, sent in the call to
    ///   [`Erc721::_check_on_erc721_received`].
    ///
    /// # Errors
    ///
    ///  * [`NonFungibleTokenError::IncorrectOwner`] - If the previous owner is
    ///    not `from`.
    ///  * [`NonFungibleTokenError::InsufficientApproval`] - If the caller does
    ///    not have the right to approve.
    ///  * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///    exist.
    ///  * [`NonFungibleTokenError::InvalidReceiver`] - If
    ///    [`IERC721Receiver::on_erc_721_received`] hasn't returned its
    ///    interface id or returned with error, or `to` is `Address::ZERO`.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: i128]`
    #[selector(name = "safeTransferFrom")]
    fn safe_transfer_from_with_data(
        e: &Env,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    );

    /// Transfers `token_id` token from `from` to `to`.
    ///
    /// WARNING: Note that the caller is responsible to confirm that the
    /// recipient is capable of receiving [`Erc721`] or else they may be
    /// permanently lost. Usage of [`Self::safe_transfer_from`] prevents loss,
    /// though the caller must understand this adds an external call which
    /// potentially creates a reentrancy vulnerability, unless it is disabled.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The address authorizing the transfer.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidReceiver`] - If `to` is
    ///   `Address::ZERO`.
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the previous owner is
    ///   not `from`.
    /// * [`NonFungibleTokenError::InsufficientApproval`] - If the caller does
    ///   not have the right to approve.
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: i128]`
    fn transfer_from(e: &Env, from: Address, to: Address, token_id: U256);

    /// Gives permission to `to` to transfer `token_id` token to another
    /// account. The approval is cleared when the token is transferred.
    ///
    /// Only a single account can be approved at a time,
    /// so approving the `Address::ZERO` clears previous approvals.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    /// * [`NonFungibleTokenError::InvalidApprover`] - If `auth` (param of
    ///   [`Erc721::_approve`]) does not have a right to approve this token.
    ///
    /// # Events
    ///
    /// * topics - `["approval", from: Address, to: Address]`
    /// * data - `[token_id: i128]`
    fn approve(e: &Env, owner: Address, to: Address, token_id: U256);

    /// Approve or remove `operator` as an operator for the caller.
    ///
    /// Operators can call [`Self::transfer_from`] or
    /// [`Self::safe_transfer_from`] for any token owned by the caller.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `operator` - Account to add to the set of authorized operators.
    /// * `approved` - Flag that determines whether or not permission will be
    ///   granted to `operator`. If true, this means `operator` will be allowed
    ///   to manage `msg::sender`'s assets.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidOperator`] - If `operator` is
    ///   `Address::ZERO`.
    ///
    /// # Events
    ///
    /// * topics - `["approval_for_all", from: Address, operator: Address]`
    /// * data - `[approved: bool]`
    fn set_approval_for_all(e: &Env, owner: Address, operator: Address, approved: bool);

    /// Returns the account approved for `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    fn get_approved(e: &Env, token_id: U256) -> Address;

    /// Returns whether the `operator` is allowed to manage all the assets of
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `owner` - Account of the token's owner.
    /// * `operator` - Account to be checked.
    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool;
}

// ################## ERRORS ##################

#[contracterror]
#[repr(u32)]
pub enum FungibleTokenError {
    /// Indicates an error related to the current balance of account from which
    /// tokens are expected to be transferred.
    InsufficientBalance = 200,
    /// Indicates a failure with the allowance mechanism when a given spender
    /// doesn't have enough allowance.
    InsufficientAllowance = 201,
    /// Indicates an invalid value for `live_until_ledger` when setting an
    /// allowance.
    InvalidLiveUntilLedger = 202,
    /// Indicates an error when an input that must be >= 0
    LessThanZero = 203,
    /// Indicates an error when an input that must be > 0
    LessThanOrEqualToZero = 204,
    /// Indicates overflow when adding two values
    MathOverflow = 205,
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
