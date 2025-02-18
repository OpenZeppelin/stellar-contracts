use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, Env};

use crate::non_fungible::{
    emit_approval, emit_approval_for_all, emit_transfer, NonFungibleTokenError,
};

// TODO: place these in another crate, called `constants`
pub const DAY_IN_LEDGERS: u32 = 17280;

pub const INSTANCE_EXTEND_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Storage key that maps to [`ApprovalData`]
#[contracttype]
pub struct ApprovalKey {
    pub token_id: u128,
}

/// Storage container for the token for which an approval is granted
/// and the ledger number at which this approval expires.
#[contracttype]
pub struct ApprovalData {
    pub approver: Address,
    pub live_until_ledger: u32,
}

/// Storage key that maps to [`ApprovalData`]
#[contracttype]
pub struct ApprovalForAllKey {
    pub owner: Address,
}

/// Storage container for the address for which an operator is granted
/// and the ledger number at which this operator expires.
#[contracttype]
pub struct ApprovalForAllData {
    pub operator: Address,
    pub approved: bool,
    pub live_until_ledger: u32,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Owner(u128),
    Balance(Address),
    Approval(ApprovalKey),
    ApprovalForAll(ApprovalForAllKey),
}

// ################## QUERY STATE ##################

/// Returns the amount of tokens held by `account`. Defaults to `0` if no
/// balance is stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address for which the balance is being queried.
pub fn balance(e: &Env, account: &Address) -> i128 {
    let key = StorageKey::Balance(account.clone());
    if let Some(balance) = e.storage().persistent().get::<_, i128>(&key) {
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
/// * `token_id` - The identifier of the token
///
/// # Errors
///
/// * [`NonFungibleTokenError::NonExistentToken`] - Occurs if the provided
///   `token_id` does not exist.
pub fn owner_of(e: &Env, token_id: u128) -> Address {
    let key = StorageKey::Owner(token_id);
    if let Some(owner) = e.storage().persistent().get::<_, Address>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);
        owner
    } else {
        // tokens always have an owner
        panic_with_error!(e, NonFungibleTokenError::NonExistentToken);
    }
}

/// Returns the address approved for `token_id` token.
pub fn get_approved(e: &Env, token_id: u128) -> Option<Address> {
    let key = StorageKey::Approval(ApprovalKey { token_id });

    if let Some(approval_data) = e.storage().temporary().get::<_, ApprovalData>(&key) {
        if approval_data.live_until_ledger < e.ledger().sequence() {
            return None; // Return None if approval expired
        }
        return Some(approval_data.approver);
    } else {
        // if there is no ApprovalData Entry for this `token_id`
        None
    }
}

/// Returns whether the `operator` is allowed to manage all assets of `owner`.
pub fn is_approved_for_all(e: &Env, owner: &Address, operator: &Address) -> bool {
    let key = StorageKey::ApprovalForAll(ApprovalForAllKey { owner: owner.clone() });

    if let Some(approval_data) = e.storage().temporary().get::<_, ApprovalForAllData>(&key) {
        if approval_data.live_until_ledger < e.ledger().sequence() {
            return false;
        }
        return approval_data.operator == *operator && approval_data.approved;
    } else {
        // if there is no ApprovalForAllData Entry for this `owner`
        false
    }
}

// ################## CHANGE STATE ##################

/// Transfers a non-fungible token (NFT) safely (reverts if the recipient cannot
/// receive the token), ensuring ownership and approval checks.
///
/// # Arguments
///
/// * `e`: The environment reference.
/// * `spender`: The address attempting to transfer the token.
/// * `from`: The current owner's address.
/// * `to`: The recipient's address.
/// * `token_id`: The identifier of the token being transferred.
///
/// # Errors
///
/// * `[`NonFungibleTokenError::IncorrectOwner`] : If the `from` address is not
///   the actual owner.
/// * `[`NonFungibleTokenError::InsufficientApproval`] : If `spender` lacks
///   transfer permission.
pub fn safe_transfer_from(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    token_id: u128,
) {
    safe_transfer_from_with_data(e, spender, from, to, token_id, Bytes::new(e));
}

// Transfers a non-fungible token (NFT) safely (reverts if the recipient cannot
// receive the token), ensuring ownership and approval
/// checks. Same as `[safe_transfer_from]`, but with additional data field.
///
/// # Arguments
///
/// * `e`: The environment reference.
/// * `spender`: The address attempting to transfer the token.
/// * `from`: The current owner's address.
/// * `to`: The recipient's address.
/// * `token_id`: The identifier of the token being transferred.
/// * `data`: Additional data with no specified format, sent in the call to
///   [`NonFungible::check_on_non_fungible_received`].
///
/// # Errors
///
/// * `[`NonFungibleTokenError::IncorrectOwner`] : If the `from` address is not
///   the actual owner.
/// * `[`NonFungibleTokenError::InsufficientApproval`] : If `spender` lacks
///   transfer permission.
pub fn safe_transfer_from_with_data(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    token_id: u128,
    _data: Bytes,
) {
    spender.require_auth();

    let owner = owner_of(e, token_id);

    // Ensure the `from` address is indeed the owner.
    if owner != *from {
        panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
    }

    // If `spender` is not the owner, they must have explicit approval.
    let is_spender_owner = *spender == owner;
    let is_spender_approved = get_approved(e, token_id) == Some(spender.clone());
    let has_spender_approval_for_all = is_approved_for_all(e, from, spender);

    if !is_spender_owner && !is_spender_approved && !has_spender_approval_for_all {
        panic_with_error!(e, NonFungibleTokenError::InsufficientApproval);
    }

    // TODO: implement the SAFE part when Receiver is implemented
    // TODO: also use the `data` field on this `Recevier`
    do_transfer(e, from, to, token_id);
}

/// Transfers a non-fungible token (NFT), ensuring ownership and approval
/// checks.
///
/// # Arguments
///
/// * `e`: The environment reference.
/// * `spender`: The address attempting to transfer the token.
/// * `from`: The current owner's address.
/// * `to`: The recipient's address.
/// * `token_id`: The identifier of the token being transferred.
///
/// # Errors
///
/// * `[`NonFungibleTokenError::IncorrectOwner`] : If the `from` address is not
///   the actual owner.
/// * `[`NonFungibleTokenError::InsufficientApproval`] : If `spender` lacks
///   transfer permission.
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
/// Use the `safe_transfer_from` variant if you also want to check things on the
/// receiver end.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u128) {
    spender.require_auth();

    let owner = owner_of(e, token_id);

    // Ensure the `from` address is indeed the owner.
    if owner != *from {
        panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
    }

    // If `spender` is not the owner, they must have explicit approval.
    let is_spender_owner = *spender == owner;
    let is_spender_approved = get_approved(e, token_id) == Some(spender.clone());
    let has_spender_approval_for_all = is_approved_for_all(e, from, spender);

    if !is_spender_owner && !is_spender_approved && !has_spender_approval_for_all {
        panic_with_error!(e, NonFungibleTokenError::InsufficientApproval);
    }

    do_transfer(e, from, to, token_id);
}

/// Approves an address to transfer a specific token
pub fn approve(e: &Env, owner: &Address, approver: &Address, token_id: u128) {
    owner.require_auth();

    // Check ownership
    let token_owner = owner_of(e, token_id);
    if token_owner != *owner {
        panic_with_error!(e, NonFungibleTokenError::InvalidApprover);
    }

    let key = StorageKey::Approval(ApprovalKey { token_id });

    let live_until_ledger = e.ledger().sequence() + INSTANCE_EXTEND_AMOUNT;

    let approval_data = ApprovalData { approver: approver.clone(), live_until_ledger };

    e.storage().temporary().set(&key, &approval_data);
    e.storage().temporary().extend_ttl(&key, INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);

    emit_approval(e, owner, approver, token_id, live_until_ledger);
}

/// Sets or removes operator approval for all tokens
pub fn set_approval_for_all(e: &Env, owner: &Address, operator: &Address, approved: bool) {
    owner.require_auth();

    let key = StorageKey::ApprovalForAll(ApprovalForAllKey { owner: owner.clone() });

    let live_until_ledger = e.ledger().sequence() + INSTANCE_EXTEND_AMOUNT;

    let approval_data =
        ApprovalForAllData { operator: operator.clone(), approved, live_until_ledger };

    e.storage().temporary().set(&key, &approval_data);
    e.storage().temporary().extend_ttl(&key, INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);

    emit_approval_for_all(e, owner, operator, approved, live_until_ledger);
}

/// Helper function to perform the actual transfer
fn do_transfer(e: &Env, from: &Address, to: &Address, token_id: u128) {
    // Update ownership
    e.storage().persistent().set(&StorageKey::Owner(token_id), to);

    // Update balances
    let from_balance = balance(e, from) - 1;
    let to_balance = balance(e, to) + 1;

    e.storage().persistent().set(&StorageKey::Balance(from.clone()), &from_balance);
    e.storage().persistent().set(&StorageKey::Balance(to.clone()), &to_balance);

    // Clear any existing approval
    let approval_key = StorageKey::Approval(ApprovalKey { token_id });
    e.storage().temporary().remove(&approval_key);

    emit_transfer(e, from, to, token_id);
}
