use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_constants::{
    BALANCE_EXTEND_AMOUNT, BALANCE_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD,
};

use crate::non_fungible::{
    emit_approval, emit_approval_for_all, emit_transfer, NonFungibleTokenError,
};

/// Storage container for the token for which an approval is granted
/// and the ledger number at which this approval expires.
#[contracttype]
pub struct ApprovalData {
    pub approved: Address,
    pub live_until_ledger: u32,
}

/// Storage container for the address for which an operator is granted
/// and the ledger number at which this operator expires.
#[contracttype]
pub struct ApprovalForAllData {
    pub operator: Address,
    pub is_approved: bool,
    pub live_until_ledger: u32,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Owner(u128),
    Balance(Address),
    Approval(u128),
    ApprovalForAll(Address),
}

// ################## QUERY STATE ##################

/// Returns the amount of tokens held by `account`. Defaults to `0` if no
/// balance is stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address for which the balance is being queried.
pub fn balance(e: &Env, account: &Address) -> u128 {
    let key = StorageKey::Balance(account.clone());
    if let Some(balance) = e.storage().persistent().get::<_, u128>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);
        balance
    } else {
        0
    }
}

/// Returns the address of the owner of the given `token_id`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - Token id as a number.
///
/// # Errors
///
/// * [`NonFungibleTokenError::NonExistentToken`] - Occurs if the provided
///   `token_id` does not exist.
pub fn owner_of(e: &Env, token_id: u128) -> Address {
    let key = StorageKey::Owner(token_id);
    if let Some(owner) = e.storage().persistent().get::<_, Address>(&key) {
        e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
        owner
    } else {
        // existing tokens always have an owner
        panic_with_error!(e, NonFungibleTokenError::NonExistentToken);
    }
}

/// Returns the address approved for the specified token:
/// * `Some(Address)` - The approved address if there is a valid, non-expired
///   approval
/// * `None` - If there is no approval or if the approval has expired
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - The identifier of the token to check approval for.
pub fn get_approved(e: &Env, token_id: u128) -> Option<Address> {
    let key = StorageKey::Approval(token_id);

    if let Some(approval_data) = e.storage().temporary().get::<_, ApprovalData>(&key) {
        if approval_data.live_until_ledger < e.ledger().sequence() {
            return None; // Return None if approval expired
        }
        Some(approval_data.approved)
    } else {
        // if there is no ApprovalData Entry for this `token_id`
        None
    }
}

/// Returns whether the operator is allowed to manage all assets of the owner:
/// * `true` - If the operator has a valid, non-expired approval for all tokens
/// * `false` - If there is no approval or if the approval has expired
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address that owns the tokens.
/// * `operator` - The address to check for approval status.
pub fn is_approved_for_all(e: &Env, owner: &Address, operator: &Address) -> bool {
    let key = StorageKey::ApprovalForAll(owner.clone());

    if let Some(approval_data) = e.storage().temporary().get::<_, ApprovalForAllData>(&key) {
        if approval_data.live_until_ledger < e.ledger().sequence() {
            return false;
        }
        approval_data.operator == *operator && approval_data.is_approved
    } else {
        // if there is no ApprovalForAllData Entry for this `owner`
        false
    }
}

// ################## CHANGE STATE ##################

/// Transfers a non-fungible token (NFT), ensuring ownership checks.
///
/// # Arguments
///
/// * `e` - The environment reference.
/// * `from` - The current owner's address.
/// * `to` - The recipient's address.
/// * `token_id` - The identifier of the token being transferred.
///
/// # Errors
///
/// * refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[token_id: u128]`
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u128) {
    from.require_auth();
    update(e, Some(from), Some(to), token_id);
    emit_transfer(e, from, to, token_id);
}

/// Transfers a non-fungible token (NFT), ensuring ownership and approval
/// checks.
///
/// # Arguments
///
/// * `e` - The environment reference.
/// * `spender` - The address attempting to transfer the token.
/// * `from` - The current owner's address.
/// * `to` - The recipient's address.
/// * `token_id` - The identifier of the token being transferred.
///
/// # Errors
///
/// * refer to [`check_spender_approval`] errors.
/// * refer to [`update`] errors.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[token_id: u128]`
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u128) {
    spender.require_auth();
    check_spender_approval(e, spender, from, token_id);
    update(e, Some(from), Some(to), token_id);
    emit_transfer(e, from, to, token_id);
}

/// Approves an address to transfer a specific token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `approver` - The address of the approver (should be `owner` or
///   `operator`).
/// * `approved` - The address receiving the approval.
/// * `token_id` - The identifier of the token to be approved.
/// * `live_until_ledger` - The ledger number at which the approval expires.
///
/// # Errors
///
/// * [`NonFungibleTokenError::InvalidApprover`] - If the owner address is not
///   the actual owner of the token.
/// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger number
///   is less than the current ledger number.
/// * refer to [`owner_of`] errors.
///
/// # Events
///
/// * topics - `["approval", owner: Address, token_id: u128]`
/// * data - `[approved: Address, live_until_ledger: u32]`
pub fn approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: u128,
    live_until_ledger: u32,
) {
    approver.require_auth();

    let owner = owner_of(e, token_id);
    if *approver != owner && !is_approved_for_all(e, &owner, approver) {
        panic_with_error!(e, NonFungibleTokenError::InvalidApprover);
    }

    if live_until_ledger < e.ledger().sequence() {
        panic_with_error!(e, NonFungibleTokenError::InvalidLiveUntilLedger);
    }

    let key = StorageKey::Approval(token_id);

    let approval_data = ApprovalData { approved: approved.clone(), live_until_ledger };

    e.storage().temporary().set(&key, &approval_data);

    let live_for = live_until_ledger - e.ledger().sequence();

    e.storage().temporary().extend_ttl(&key, live_for, live_for);

    emit_approval(e, approver, approved, token_id, live_until_ledger);
}

/// Sets or removes operator approval for managing all tokens owned by the
/// owner.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address granting approval for all their tokens.
/// * `operator` - The address being granted or revoked approval.
/// * `is_approved` - If true, grants approval; if false, revokes approval.
/// * `live_until_ledger` - The ledger number at which the approval expires.
///
/// # Errors
///
/// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger number
///   is less than the current ledger number.
///
/// # Events
///
/// * topics - `["approval", owner: Address]`
/// * data - `[operator: Address, is_approved: bool, live_until_ledger: u32]`
pub fn set_approval_for_all(
    e: &Env,
    owner: &Address,
    operator: &Address,
    is_approved: bool,
    live_until_ledger: u32,
) {
    owner.require_auth();

    if live_until_ledger < e.ledger().sequence() {
        panic_with_error!(e, NonFungibleTokenError::InvalidLiveUntilLedger);
    }

    let key = StorageKey::ApprovalForAll(owner.clone());

    let approval_data =
        ApprovalForAllData { operator: operator.clone(), is_approved, live_until_ledger };

    e.storage().temporary().set(&key, &approval_data);

    let live_for = live_until_ledger - e.ledger().sequence();

    e.storage().temporary().extend_ttl(&key, live_for, live_for);

    emit_approval_for_all(e, owner, operator, is_approved, live_until_ledger);
}

/// Low-level function for handling transfers for NFTs, but doesn't
/// handle authorization. Updates ownership records, adjusts balances,
/// and clears existing approvals.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The address of the current token owner.
/// * `to` - The address of the token recipient.
/// * `token_id` - The identifier of the token to be transferred.
///
/// # Errors
///
/// * [`NonFungibleTokenError::IncorrectOwner`] - If the `from` address is not
///   the owner of the token.
/// * [`NonFungibleTokenError::MathOverflow`] - If the balance of the `to` would
///   overflow.
pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, token_id: u128) {
    if let Some(from_address) = from {
        let owner = owner_of(e, token_id);

        // Ensure the `from` address is indeed the owner.
        if owner != *from_address {
            panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
        }

        // Update the balance of the `from` address
        // No need to check for underflow here, as `owner` cannot have `0` balance,
        // and if `from_balance` is not the `owner`, we have already panicked above.
        let from_balance = balance(e, from_address) - 1;
        e.storage().persistent().set(&StorageKey::Balance(from_address.clone()), &from_balance);

        // Clear any existing approval
        let approval_key = StorageKey::Approval(token_id);
        e.storage().temporary().remove(&approval_key);
    } else {
        // nothing to do for the `None` case, since we don't track
        // `total_supply`
    }

    if let Some(to_address) = to {
        // Update the balance of the `to` address
        let Some(to_balance) = balance(e, to_address).checked_add(1) else {
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().persistent().set(&StorageKey::Balance(to_address.clone()), &to_balance);

        // Set the new owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), to_address);
    } else {
        // Burning: `to` is None
        e.storage().persistent().remove(&StorageKey::Owner(token_id));
    }
}

/// Low-level function for checking if the `spender` has enough approval.
/// Panics if the approval check fails.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The address attempting to transfer the token.
/// * `owner` - The address of the current token owner.
///
/// # Errors
/// * [`NonFungibleTokenError::InsufficientApproval`] - If the `spender` don't
///   enough approval.
pub fn check_spender_approval(e: &Env, spender: &Address, owner: &Address, token_id: u128) {
    // If `spender` is not the owner, they must have explicit approval.
    let is_spender_owner = spender == owner;
    let is_spender_approved = get_approved(e, token_id) == Some(spender.clone());
    let has_spender_approval_for_all = is_approved_for_all(e, owner, spender);

    if !is_spender_owner && !is_spender_approved && !has_spender_approval_for_all {
        panic_with_error!(e, NonFungibleTokenError::InsufficientApproval);
    }
}
