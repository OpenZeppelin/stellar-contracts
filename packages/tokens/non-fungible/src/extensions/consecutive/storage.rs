use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use super::emit_consecutive_mint;
use crate::{
    burnable::emit_burn,
    emit_transfer,
    sequential::{increment_token_id, next_token_id},
    storage::{approve_for_owner, check_spender_approval, decrease_balance, increase_balance},
    NonFungibleTokenError,
};

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Approval(u32),
    Owner(u32),
    BurntToken(u32),
}

// ################## QUERY STATE ##################

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
pub fn owner_of(e: &Env, token_id: u32) -> Address {
    let max = next_token_id(e);
    let is_burnt = e.storage().persistent().get(&StorageKey::BurntToken(token_id)).unwrap_or(false);

    if token_id >= max || is_burnt {
        panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
    }

    //e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD,
    // OWNER_EXTEND_AMOUNT);
    (0..=token_id)
        .rev()
        .map(StorageKey::Owner)
        .find_map(|key| e.storage().persistent().get::<_, Address>(&key))
        .unwrap_or_else(|| panic_with_error!(&e, NonFungibleTokenError::NonExistentToken))
}

// ################## CHANGE STATE ##################

pub fn batch_mint(e: &Env, to: &Address, amount: u32) -> u32 {
    let next_id = increment_token_id(e, amount);

    e.storage().persistent().set(&StorageKey::Owner(next_id), &to);

    increase_balance(e, to, amount);

    let last_id = next_id + amount - 1;
    emit_consecutive_mint(e, to, next_id, last_id);

    // return the last minted id
    last_id
}

pub fn burn(e: &Env, from: &Address, token_id: u32) {
    from.require_auth();
    update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);

    e.storage().persistent().set(&StorageKey::BurntToken(token_id), &true);
    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
}

pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
    spender.require_auth();
    check_spender_approval(e, spender, from, token_id);
    update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);

    e.storage().persistent().set(&StorageKey::BurntToken(token_id), &true);
    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
}

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
/// * data - `[token_id: u32]`
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
    from.require_auth();

    update(e, Some(from), Some(to), token_id);
    emit_transfer(e, from, to, token_id);

    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
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
/// * data - `[token_id: u32]`
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
    spender.require_auth();

    check_spender_approval(e, spender, from, token_id);

    update(e, Some(from), Some(to), token_id);
    emit_transfer(e, from, to, token_id);

    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
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
/// * topics - `["approve", owner: Address, token_id: u32]`
/// * data - `[approved: Address, live_until_ledger: u32]`
pub fn approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: u32,
    live_until_ledger: u32,
) {
    approver.require_auth();

    let owner = owner_of(e, token_id);
    approve_for_owner(e, &owner, approver, approved, token_id, live_until_ledger);
}

pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, token_id: u32) {
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

fn set_owner_for_next_token(e: &Env, owner: &Address, token_id: u32) {
    let next_token_id = token_id + 1;
    let has_owner = e.storage().persistent().has(&StorageKey::Owner(next_token_id));
    let is_burnt =
        e.storage().persistent().get(&StorageKey::BurntToken(next_token_id)).unwrap_or(false);

    if !has_owner && !is_burnt {
        e.storage().persistent().set(&StorageKey::Owner(next_token_id), owner);
    }
}
