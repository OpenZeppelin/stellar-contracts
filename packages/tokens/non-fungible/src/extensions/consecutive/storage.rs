use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{emit_transfer, storage::check_spender_approval, NonFungibleTokenError};

use super::INonFungibleConsecutive;

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Balance(Address),
    Approval(u32),
    Owner(u32),
    TokenIdCounter,
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
pub fn owner_of<T: INonFungibleConsecutive>(e: &Env, token_id: u32) -> Address {
    let max = T::next_token_id(e);
    let is_burnt = e.storage().persistent().get(&StorageKey::BurntToken(token_id)).unwrap_or(false);

    if token_id >= max || is_burnt {
        panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
    }

    //e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);
    (0..=token_id)
        .rev()
        .map(StorageKey::Owner)
        .find_map(|key| e.storage().persistent().get::<_, Address>(&key))
        .unwrap_or_else(|| panic_with_error!(&e, NonFungibleTokenError::NonExistentToken))
}

// ################## CHANGE STATE ##################

pub fn batch_mint<T: INonFungibleConsecutive>(e: &Env, to: &Address, amount: u32) {
    let next_id = T::increment_token_id_by(e, amount);

    e.storage().persistent().set(&StorageKey::Owner(next_id), &to);

    T::increase_balance(e, to.clone(), amount);
}

pub fn burn<T: INonFungibleConsecutive>(e: &Env, from: &Address, token_id: u32) {
    from.require_auth();
    update::<T>(e, Some(from), None, token_id);

    e.storage().persistent().set(&StorageKey::BurntToken(token_id), &true);
}

pub fn burn_from<T: INonFungibleConsecutive>(
    e: &Env,
    spender: &Address,
    from: &Address,
    token_id: u32,
) {
    spender.require_auth();
    check_spender_approval(e, spender, from, token_id);
    burn::<T>(e, from, token_id);
}

//pub fn burn(e: &Env, from: &Address, token_id: u32) {}

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
pub fn transfer<T: INonFungibleConsecutive>(e: &Env, from: &Address, to: &Address, token_id: u32) {
    from.require_auth();
    update::<T>(e, Some(from), Some(to), token_id);
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
/// * data - `[token_id: u32]`
///
/// # Notes
///
/// **IMPORTANT**: If the recipient is unable to receive, the NFT may get lost.
pub fn transfer_from<T: INonFungibleConsecutive>(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    token_id: u32,
) {
    spender.require_auth();
    check_spender_approval(e, spender, from, token_id);
    update::<T>(e, Some(from), Some(to), token_id);
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
/// * topics - `["approve", owner: Address, token_id: u32]`
/// * data - `[approved: Address, live_until_ledger: u32]`
pub fn approve(
    _e: &Env,
    _approver: &Address,
    _approved: &Address,
    _token_id: u32,
    _live_until_ledger: u32,
) {
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
pub fn update<T: INonFungibleConsecutive>(
    e: &Env,
    from: Option<&Address>,
    to: Option<&Address>,
    token_id: u32,
) {
    let owner = owner_of::<T>(e, token_id);

    if let Some(from_address) = from {
        // Ensure the `from` address is indeed the owner.
        if owner != *from_address {
            panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
        }

        T::decrease_balance(e, from_address.clone(), 1);

        // TODO: change to T::remove_approve(token_id) ??
        // Clear any existing approval
        let approval_key = StorageKey::Approval(token_id);
        e.storage().temporary().remove(&approval_key);
    } else {
        // nothing to do for the `None` case, since we don't track
        // `total_supply`
    }

    if let Some(to_address) = to {
        // Update the balance of the `to` address
        T::increase_balance(e, to_address.clone(), 1);

        // Set the new owner
        e.storage().persistent().set(&StorageKey::Owner(token_id), &to_address);
    } else {
        // Burning: `to` is None
        e.storage().persistent().remove(&StorageKey::Owner(token_id));
    }

    // Set the next token to prev owner
    set_owner_for_next_token(e, &owner, token_id);
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
