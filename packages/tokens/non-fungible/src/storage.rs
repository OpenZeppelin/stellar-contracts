use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map};
use stellar_constants::{
    BALANCE_EXTEND_AMOUNT, BALANCE_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD,
};

use crate::non_fungible::{
    emit_approve, emit_approve_for_all, emit_transfer, Balance, NonFungibleTokenError, TokenId,
};

/// Storage container for the token for which an approval is granted
/// and the ledger number at which this approval expires.
#[contracttype]
pub struct ApprovalData {
    pub approved: Address,
    pub live_until_ledger: u32,
}

/// Storage container for multiple operators and their expiration ledgers.
#[contracttype]
pub struct ApprovalForAllData {
    pub operators: Map<Address /* operator */, u32 /* live_until_ledger */>,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Owner(TokenId),
    Balance(Address),
    Approval(TokenId),
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
pub fn balance(e: &Env, account: &Address) -> Balance {
    let key = StorageKey::Balance(account.clone());
    if let Some(balance) = e.storage().persistent().get::<_, Balance>(&key) {
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
pub fn owner_of(e: &Env, token_id: TokenId) -> Address {
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
pub fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
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

    // Retrieve the approval data for the owner
    if let Some(approval_data) = e.storage().temporary().get::<_, ApprovalForAllData>(&key) {
        // Check if the operator exists and if their approval is valid (non-expired)
        if let Some(expiry) = approval_data.operators.get(operator.clone()) {
            if expiry >= e.ledger().sequence() {
                return true;
            }
        }
    }

    // If no operator with a valid approval
    false
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
/// * data - `[token_id: TokenId]`
///
/// # Notes
///
/// * Authorization for `from` is required.
/// * **IMPORTANT**: If the recipient is unable to receive, the NFT may get
///   lost.
pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: TokenId) {
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
/// * data - `[token_id: TokenId]`
///
/// # Notes
///
/// * Authorization for `spender` is required.
/// * **IMPORTANT**: If the recipient is unable to receive, the NFT may get
///   lost.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: TokenId) {
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
/// * refer to [`owner_of`] errors.
/// * refer to [`approve_for_owner`] errors.
///
/// # Events
///
/// * topics - `["approve", owner: Address, token_id: TokenId]`
/// * data - `[approved: Address, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for `approver` is required.
pub fn approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: TokenId,
    live_until_ledger: u32,
) {
    approver.require_auth();

    let owner = owner_of(e, token_id);
    approve_for_owner(e, &owner, approver, approved, token_id, live_until_ledger);
}

/// Sets or removes operator approval for managing all tokens owned by the
/// owner.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address granting approval for all their tokens.
/// * `operator` - The address being granted or revoked approval.
/// * `live_until_ledger` - The ledger number at which the allowance expires. If
///   `live_until_ledger` is `0`, the approval is revoked.
///
/// # Errors
///
/// * [`NonFungibleTokenError::InvalidLiveUntilLedger`] - If the ledger number
///   is less than the current ledger number.
///
/// # Events
///
/// * topics - `["approve", owner: Address]`
/// * data - `[operator: Address, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for `owner` is required.
pub fn approve_for_all(e: &Env, owner: &Address, operator: &Address, live_until_ledger: u32) {
    owner.require_auth();

    let key = StorageKey::ApprovalForAll(owner.clone());

    // If revoking approval (live_until_ledger == 0)
    if live_until_ledger == 0 {
        if let Some(mut approval_data) = e.storage().temporary().get::<_, ApprovalForAllData>(&key)
        {
            approval_data.operators.remove(operator.clone());
            e.storage().temporary().set(&key, &approval_data);
        }
        emit_approve_for_all(e, owner, operator, live_until_ledger);
        return;
    }

    // If the provided ledger number is invalid (less than the current ledger
    // number)
    if live_until_ledger < e.ledger().sequence() {
        panic_with_error!(e, NonFungibleTokenError::InvalidLiveUntilLedger);
    }

    // Retrieve or initialize the approval data
    let mut approval_data = e
        .storage()
        .temporary()
        .get::<_, ApprovalForAllData>(&key)
        .unwrap_or_else(|| ApprovalForAllData { operators: Map::new(e) });

    // Set the operator's expiration ledger
    approval_data.operators.set(operator.clone(), live_until_ledger);

    // Update the storage
    e.storage().temporary().set(&key, &approval_data);

    // Update the TTL based on the expiration ledger
    let live_for = live_until_ledger - e.ledger().sequence();
    e.storage().temporary().extend_ttl(&key, live_for, live_for);

    emit_approve_for_all(e, owner, operator, live_until_ledger);
}

/// Low-level function for handling transfers, mints and burns of an NFT,
/// without handling authorization. Updates ownership records, adjusts balances,
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
/// * refer to [`owner_of`] errors.
/// * refer to [`decrease_balance`] errors.
/// * refer to [`increase_balance`] errors.
pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, token_id: TokenId) {
    if let Some(from_address) = from {
        let owner = owner_of(e, token_id);

        // Ensure the `from` address is indeed the owner.
        if owner != *from_address {
            panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
        }

        decrease_balance(e, from_address, 1);

        // Clear any existing approval
        let approval_key = StorageKey::Approval(token_id);
        e.storage().temporary().remove(&approval_key);
    } else {
        // nothing to do for the `None` case, since we don't track
        // `total_supply`
    }

    if let Some(to_address) = to {
        increase_balance(e, to_address, 1);

        // Set the new owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), to_address);
    } else {
        // Burning: `to` is None
        e.storage().persistent().remove(&StorageKey::Owner(token_id));
    }
}

/// Low-level function for approving `token_id` without checking its ownership
/// and without handling authorization.
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
pub fn approve_for_owner(
    e: &Env,
    owner: &Address,
    approver: &Address,
    approved: &Address,
    token_id: TokenId,
    live_until_ledger: u32,
) {
    if approver != owner && !is_approved_for_all(e, owner, approver) {
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

    emit_approve(e, approver, approved, token_id, live_until_ledger);
}

/// Low-level function for checking if the `spender` has enough approval prior a
/// transfer, without checking ownership of `token_id` and without handling
/// authorization.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The address attempting to transfer the token.
/// * `owner` - The address of the current token owner.
/// * `token_id` - The identifier of the token to be transferred.
///
/// # Errors
/// * [`NonFungibleTokenError::InsufficientApproval`] - If the `spender` don't
///   enough approval.
pub fn check_spender_approval(e: &Env, spender: &Address, owner: &Address, token_id: TokenId) {
    // If `spender` is not the owner, they must have explicit approval.
    let is_spender_owner = spender == owner;
    let is_spender_approved = get_approved(e, token_id) == Some(spender.clone());
    let has_spender_approval_for_all = is_approved_for_all(e, owner, spender);

    if !is_spender_owner && !is_spender_approved && !has_spender_approval_for_all {
        panic_with_error!(e, NonFungibleTokenError::InsufficientApproval);
    }
}

/// Low-level function for increasing the balance of `to`, without handling
/// authorization.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address whose balance gets increased.
/// * `amount` - The amount by which the balance gets increased.
///
/// # Errors
///
/// * [`NonFungibleTokenError::MathOverflow`] - If the balance of the `to` would
///   overflow.
pub fn increase_balance(e: &Env, to: &Address, amount: TokenId) {
    let Some(balance) = balance(e, to).checked_add(amount) else {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    };
    e.storage().persistent().set(&StorageKey::Balance(to.clone()), &balance);
}

/// Low-level function for decreasing the balance of `to`, without handling
/// authorization.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address whose balance gets decreased.
/// * `amount` - The amount by which the balance gets decreased.
///
/// # Errors
///
/// * [`NonFungibleTokenError::MathOverflow`] - If the balance of the `from`
///   would overflow.
pub fn decrease_balance(e: &Env, from: &Address, amount: TokenId) {
    let Some(balance) = balance(e, from).checked_sub(amount) else {
        panic_with_error!(e, NonFungibleTokenError::MathOverflow);
    };
    e.storage().persistent().set(&StorageKey::Balance(from.clone()), &balance);
}
