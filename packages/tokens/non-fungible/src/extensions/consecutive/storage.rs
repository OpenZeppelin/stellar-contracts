use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    burnable::emit_burn, emit_transfer, storage2::check_spender_approval, NonFungibleInternal,
    NonFungibleTokenError,
};

use super::{emit_consecutive_mint, NonFungibleConsecutive};

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
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
pub fn owner_of<T: NonFungibleConsecutive>(e: &Env, token_id: u32) -> Address {
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

pub fn batch_mint<T: NonFungibleConsecutive>(e: &Env, to: &Address, amount: u32) -> u32 {
    let next_id = T::increment_token_id(e, amount);

    e.storage().persistent().set(&StorageKey::Owner(next_id), &to);

    T::increase_balance(e, to.clone(), amount);

    let last_id = next_id + amount - 1;
    emit_consecutive_mint(e, to, next_id, last_id);

    // return the last minted id
    last_id
}

pub fn burn<T: NonFungibleConsecutive>(e: &Env, from: &Address, token_id: u32) {
    from.require_auth();

    T::update(e, Some(from), None, token_id);
    e.storage().persistent().set(&StorageKey::BurntToken(token_id), &true);
    emit_burn(e, from, token_id);

    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
}

pub fn burn_from<T: NonFungibleConsecutive>(
    e: &Env,
    spender: &Address,
    from: &Address,
    token_id: u32,
) {
    spender.require_auth();

    check_spender_approval::<T>(e, spender, from, token_id);

    T::update(e, Some(from), None, token_id);
    e.storage().persistent().set(&StorageKey::BurntToken(token_id), &true);
    emit_burn(e, from, token_id);

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
pub fn transfer<T: NonFungibleConsecutive>(e: &Env, from: &Address, to: &Address, token_id: u32) {
    from.require_auth();

    T::update(e, Some(from), Some(to), token_id);
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
pub fn transfer_from<T: NonFungibleConsecutive>(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    token_id: u32,
) {
    spender.require_auth();

    check_spender_approval::<T>(e, spender, from, token_id);

    T::update(e, Some(from), Some(to), token_id);
    emit_transfer(e, from, to, token_id);

    // Set the next token to prev owner
    set_owner_for_next_token(e, from, token_id);
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
