use soroban_sdk::{contractclient, contracterror, symbol_short, Address, Env, Symbol};

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
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::balance()`] when implementing this function.
    fn balance(e: &Env, owner: Address) -> u128;

    /// Returns the owner of the `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::owner_of()`] when implementing this
    /// function.
    fn owner_of(e: &Env, token_id: u128) -> Address;

    /// Transfers `token_id` token from `from` to `to`.
    ///
    /// WARNING: Note that the caller is responsible to confirm that the
    /// recipient is capable of receiving the `Non-Fungible` or else the NFT
    /// may be permanently lost.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - Account of the sender.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the previous owner is
    ///   not `from`.
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::transfer()`] when implementing this
    /// function.
    fn transfer(e: &Env, from: Address, to: Address, token_id: u128);

    /// Transfers `token_id` token from `from` to `to` with authorization from
    /// `spender`.
    ///
    /// Unlike `transfer()`, which is used when the token owner initiates the
    /// transfer, `transfer_from()` allows an approved third party
    /// (`spender`) to transfer the token on behalf of the owner. This
    /// function includes an on-chain check to verify that `spender` has the
    /// necessary approval.
    ///
    /// WARNING: Note that the caller is responsible to confirm that the
    /// recipient is capable of receiving the `Non-Fungible` or else the NFT
    /// may be permanently lost.
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
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the previous owner is
    ///   not `from`.
    /// * [`NonFungibleTokenError::UnauthorizedTransfer`] - If the caller does
    ///   not have the right to approve.
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u128]`
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::transfer_from()`] when implementing this
    /// function.
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: u128);

    /// Gives permission to `to` to transfer `token_id` token to another
    /// account. The approval is cleared when the token is transferred.
    ///
    /// Only a single account can be approved at a time.
    /// To remove an approval, the owner can approve their own address,
    /// effectively removing the previous approved address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `to` - Account of the recipient.
    /// * `token_id` - Token id as a number.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
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
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::approve()`] when implementing this
    /// function.
    fn approve(e: &Env, owner: Address, to: Address, token_id: u128, live_until_ledger: u32);

    /// Approve or remove `operator` as an operator for the owner.
    ///
    /// Operators can call [`Self::transfer_from`] for any token owned by the
    /// caller.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `operator` - Account to add to the set of authorized operators.
    /// * `approved` - Flag that determines whether or not permission will be
    ///   granted to `operator`. If true, this means `operator` will be allowed
    ///   to manage `owner`'s assets.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
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
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::set_approval_for_all()`] when implementing
    /// this function.
    fn set_approval_for_all(
        e: &Env,
        owner: Address,
        operator: Address,
        approved: bool,
        live_until_ledger: u32,
    );

    /// Returns the account approved for `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonexistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::get_approved()`] when implementing this
    /// function.
    fn get_approved(e: &Env, token_id: u128) -> Option<Address>;

    /// Returns whether the `operator` is allowed to manage all the assets of
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `operator` - Account to be checked.
    ///
    /// # Notes
    ///
    /// We recommend using [`crate::is_approved_for_all()`] when implementing
    /// this function.
    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool;

    /// Returns the token collection name.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn name(e: &Env) -> String;

    /// Returns the token collection symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn symbol(e: &Env) -> String;

    /// Returns the Uniform Resource Identifier (URI) for `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token id as a number.
    ///
    /// # Notes
    ///
    /// If the token does not exist, this function is expected to panic.
    fn token_uri(e: &Env, token_id: u128) -> String;
}

// ################## ERRORS ##################

#[contracterror]
#[repr(u32)]
pub enum NonFungibleTokenError {
    /// Indicates a non-existent `token_id`.
    NonExistentToken = 300,
    /// Indicates an error related to the ownership over a particular token.
    /// Used in transfers.
    IncorrectOwner = 301,
    /// Indicates a failure with the token `sender`. Used in transfers.
    InvalidSender = 302,
    /// Indicates a failure with the token `receiver`. Used in transfers.
    InvalidReceiver = 303,
    /// Indicates a failure with the `operator`’s approval. Used in transfers.
    UnauthorizedTransfer = 304,
    /// Indicates a failure with the `approver` of a token to be approved. Used
    /// in approvals.
    InvalidApprover = 305,
    /// Indicates a failure with the `operator` to be approved. Used in
    /// approvals.
    InvalidOperator = 306,
    /// Indicates an invalid value for `live_until_ledger` when setting
    /// approvals.
    InvalidLiveUntilLedger = 307,
    /// Indicates overflow when adding two values
    MathOverflow = 308,
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
/// * `approved` - Address of the approved.
/// * `token_id` - The identifier of the transferred token.
///
/// # Events
///
/// * topics - `["approval", owner: Address, token_id: u128]`
/// * data - `[approved: Address, live_until_ledger: u32]`
pub fn emit_approval(
    e: &Env,
    owner: &Address,
    approved: &Address,
    token_id: u128,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approval"), owner, token_id);
    e.events().publish(topics, (approved, live_until_ledger))
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
/// * topics - `["approval", owner: Address]`
/// * data - `[operator: Address, approved: bool, live_until_ledger: u32]`
pub fn emit_approval_for_all(
    e: &Env,
    owner: &Address,
    operator: &Address,
    approved: bool,
    live_until_ledger: u32,
) {
    let topics = (Symbol::new(e, "approval_for_all"), owner);
    e.events().publish(topics, (operator, approved, live_until_ledger))
}
