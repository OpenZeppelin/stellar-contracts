use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Bytes, Env};

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
    fn balance_of(e: &Env, owner: Address) -> i128;

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
    fn owner_of(e: &Env, token_id: u128) -> Address;

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
    /// * data - `[token_id: u128]`
    fn safe_transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u128);

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
    /// * data - `[token_id: u128]`
    fn safe_transfer_from_with_data(
        e: &Env,
        from: Address,
        to: Address,
        token_id: u128,
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
    /// * data - `[token_id: u128]`
    fn transfer_from(e: &Env, from: Address, to: Address, token_id: u128);

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
    /// * data - `[token_id: u128]`
    fn approve(e: &Env, owner: Address, to: Address, token_id: u128);

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
    fn get_approved(e: &Env, token_id: u128) -> Address;

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
pub enum NonFungibleTokenError {
    /// Indicates a non-existent `token_id`.
    NonexistentToken = 300,
    /// Indicates an error related to the ownership over a particular token.
    /// Used in transfers.
    IncorrectOwner = 301,
    /// Indicates a failure with the token `sender`. Used in transfers.
    InvalidSender = 302,
    /// Indicates a failure with the token `receiver`. Used in transfers.
    InvalidReceiver = 303,
    /// Indicates a failure with the `operator`’s approval. Used in transfers.
    InsufficientApproval = 304,
    /// Indicates a failure with the `approver` of a token to be approved. Used
    /// in approvals.
    InvalidApprover = 305,
    /// Indicates a failure with the `operator` to be approved. Used in
    /// approvals.
    InvalidOperator = 306,
}

// ################## EVENTS ##################

/// Emits an event indicating a transfer of token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the token.
/// * `to` - The address receiving the transferred token.
/// * `token_id` - The identifier of the transferred token.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[token_id: u128]`
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, token_id: u128) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, token_id)
}

/// Emits an event when `owner` enables `approved` to manage the `token_id`
/// token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - Address of the owner of the token.
/// * `approver` - Address of the approver.
/// * `token_id` - The identifier of the transferred token.
///
/// # Events
///
/// * topics - `["approval", owner: Address, approver: Address]`
/// * data - `[token_id: u128, live_until_ledger: u32]`
pub fn emit_approval(
    e: &Env,
    owner: &Address,
    approver: &Address,
    token_id: u128,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approval"), owner, approver);
    e.events().publish(topics, (token_id, live_until_ledger))
}

/// Emits an event when `owner` enables `approved` to manage the `token_id`
/// token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - Address of the owner of the token.
/// * `operator` - Address of an operator that will manage operations on the
///   token.
/// * `approved` - Whether or not permission has been granted. If true, this
///   means `operator` will be allowed to manage `owner`'s assets.
///
/// # Events
///
/// * topics - `["approval", owner: Address, operator: Address]`
/// * data - `[approved: bool, live_until_ledger: u32]`
pub fn emit_approval_for_all(
    e: &Env,
    owner: &Address,
    operator: &Address,
    approved: bool,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approval"), owner, operator);
    e.events().publish(topics, (approved, live_until_ledger))
}
