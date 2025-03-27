#[cfg(feature = "token_u256")]
use soroban_sdk::U256;
use soroban_sdk::{contracterror, symbol_short, Address, Env, String, Symbol};

use crate::ContractOverrides;

#[cfg(feature = "token_u32")]
pub type TokenId = u32;

#[cfg(feature = "token_u64")]
pub type TokenId = u64;

#[cfg(feature = "token_u128")]
pub type TokenId = u128;

#[cfg(feature = "token_u256")]
pub type TokenId = U256;

// one user can possess at most `TokenId` cap of tokens.
pub type Balance = TokenId;

/// Vanilla NonFungible Token Trait
///
/// The `NonFungibleToken` trait defines the core functionality for non-fungible
/// tokens. It provides a standard interface for managing
/// transfers and approvals associated with non-fungible tokens.
pub trait NonFungibleToken {
    /// Helper type that allows us to override some of the functionality of the
    /// base trait based on the extensions implemented. You should use
    /// `BaseContract` as the type if you are not using `Enumerable` or
    /// `Consecutive` extensions.
    type ContractType: ContractOverrides;

    /// Returns the number of tokens in `owner`'s account.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    fn balance(e: &Env, owner: Address) -> Balance {
        crate::balance(e, &owner)
    }

    /// Returns the owner of the `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Notes
    ///
    /// This function's behavior is shaped by the extensions implemented.
    /// It should be configured via the `ContractBehavior` helper trait.
    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Self::ContractType::owner_of(e, token_id)
    }

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
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the current owner
    ///   (before calling this function) is not `from`.
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: TokenId]`
    ///
    /// # Notes
    ///
    /// This function's behavior is shaped by the extensions implemented.
    /// It should be configured via the `ContractBehavior` helper trait.
    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer(e, from, to, token_id);
    }

    /// Transfers `token_id` token from `from` to `to` by using `spender`s
    /// approval.
    ///
    /// Unlike `transfer()`, which is used when the token owner initiates the
    /// transfer, `transfer_from()` allows an approved third party
    /// (`spender`) to transfer the token on behalf of the owner. This
    /// function verifies that `spender` has the necessary approval.
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
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the current owner
    ///   (before calling this function) is not `from`.
    /// * [`NonFungibleTokenError::InsufficientApproval`] - If the spender does
    ///   not have a valid approval.
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: TokenId]`
    ///
    /// # Notes
    ///
    /// This function's behavior is shaped by the extensions implemented.
    /// It should be configured via the `ContractBehavior` helper trait.
    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer_from(e, spender, from, to, token_id);
    }

    /// Gives permission to `approved` to transfer `token_id` token to another
    /// account. The approval is cleared when the token is transferred.
    ///
    /// Only a single account can be approved at a time for a `token_id`.
    /// To remove an approval, the approver can approve their own address,
    /// effectively removing the previous approved address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `approver` - The address of the approver (should be `owner` or
    ///   `operator`).
    /// * `approved` - The address receiving the approval.
    /// * `token_id` - Token id as a number.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    /// * [`NonFungibleTokenError::InvalidApprover`] - If the owner address is
    ///   not the actual owner of the token.
    /// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger
    ///   number is less than the current ledger number.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, to: Address]`
    /// * data - `[token_id: TokenId, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// This function's behavior is shaped by the extensions implemented.
    /// It should be configured via the `ContractBehavior` helper trait.
    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Self::ContractType::approve(e, approver, approved, token_id, live_until_ledger);
    }

    /// Approve or remove `operator` as an operator for the owner.
    ///
    /// Operators can call `transfer_from()` for any token held by `owner`,
    /// and call `approve()` on behalf of `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `owner` - The address holding the tokens.
    /// * `operator` - Account to add to the set of authorized operators.
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires. If `live_until_ledger` is `0`, the approval is revoked.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger
    ///   number is less than the current ledger number.
    ///
    /// # Events
    ///
    /// * topics - `["approve_for_all", from: Address]`
    /// * data - `[operator: Address, live_until_ledger: u32]`
    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        crate::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    /// Returns the account approved for `token_id` token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token id as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If the token does not
    ///   exist.
    fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
        crate::get_approved(e, token_id)
    }

    /// Returns whether the `operator` is allowed to manage all the assets of
    /// `owner`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `operator` - Account to be checked.
    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        crate::is_approved_for_all(e, &owner, &operator)
    }

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
    fn token_uri(e: &Env, token_id: TokenId) -> String;
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
    /// Indicates a failure with the `operator`’s approval. Used in transfers.
    InsufficientApproval = 302,
    /// Indicates a failure with the `approver` of a token to be approved. Used
    /// in approvals.
    InvalidApprover = 303,
    /// Indicates an invalid value for `live_until_ledger` when setting
    /// approvals.
    InvalidLiveUntilLedger = 304,
    /// Indicates overflow when adding two values
    MathOverflow = 305,
    /// Indicates all possible `token_id`s are already in use.
    TokenIDsAreDepleted = 306,
    /// Indicates a token with given `token_id` already exists.
    TokenIDInUse = 307,
    /// Indicates the token does not exist in owner's list.
    TokenNotFoundInOwnerList = 308,
    /// Indicates the token does not exist in global list.
    TokenNotFoundInGlobalList = 309,
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
/// * data - `[token_id: TokenId]`
pub fn emit_transfer(e: &Env, from: &Address, to: &Address, token_id: TokenId) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, token_id)
}

/// Emits an event when `approver` enables `approved` to manage the `token_id`
/// token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `approver` - The address of the approver (should be `owner` or
///   `operator`).
/// * `approved` - Address of the approved.
/// * `token_id` - The identifier of the transferred token.
/// * `live_until_ledger` - The ledger number at which the approval expires.
///
/// # Events
///
/// * topics - `["approve", owner: Address, token_id: TokenId]`
/// * data - `[approved: Address, live_until_ledger: u32]`
pub fn emit_approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: TokenId,
    live_until_ledger: u32,
) {
    let topics = (symbol_short!("approve"), approver, token_id);
    e.events().publish(topics, (approved, live_until_ledger))
}

/// Emits an event when `owner` enables `operator` to manage the `token_id`
/// token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - Address of the owner of the token.
/// * `operator` - Address of an operator that will manage operations on the
///   token.
/// * `live_until_ledger` - The ledger number at which the allowance expires. If
///   `live_until_ledger` is `0`, the approval is revoked.
///
/// # Events
///
/// * topics - `["approve_for_all", owner: Address]`
/// * data - `[operator: Address, live_until_ledger: u32]`
pub fn emit_approve_for_all(e: &Env, owner: &Address, operator: &Address, live_until_ledger: u32) {
    let topics = (Symbol::new(e, "approve_for_all"), owner);
    e.events().publish(topics, (operator, live_until_ledger))
}
