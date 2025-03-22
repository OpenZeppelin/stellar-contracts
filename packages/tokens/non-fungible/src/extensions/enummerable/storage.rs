use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{sequential::NonFungibleSequential, storage::balance, NonFungibleTokenError};

use super::{Enumeration, NonFungibleEnumerable, NonSequential, Sequential};

/// Storage key that maps to [`AllowanceData`]
#[contracttype]
pub struct OwnerTokensKey {
    pub owner: Address,
    pub index: u32,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    TotalSupply,
    OwnerTokens(OwnerTokensKey),
    OwnerTokensIndex(/* token_id */ u32),
    GlobalTokens(/* index */ u32),
    GlobalTokensIndex(/* token_id */ u32),
}

// ################## QUERY STATE ##################

/// Returns the total amount of tokens stored by the contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn total_supply<T: NonFungibleEnumerable>(e: &Env) -> u32 {
    e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
}

/// Returns the `token_id` owned by `owner` at a given `index` in the
/// owner's local list. Use along with
/// [`crate::NonFungibleToken::balance()`] to enumerate all of `owner`'s
/// tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - Account of the token's owner.
/// * `index` - Index of the token in the owner's local list.
///
/// # Errors
///
/// * [`NonFungibleTokenError::TokenNotFoundInOwnerList`] - When the token ID is
///   not found in the owner's enumeration.
pub fn get_owner_token_id<T: NonFungibleEnumerable>(e: &Env, owner: &Address, index: u32) -> u32 {
    let key = StorageKey::OwnerTokens(OwnerTokensKey { owner: owner.clone(), index });
    let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
        panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInOwnerList);
    };

    token_id
}

/// Returns the `token_id` at a given `index` in the global token list.
/// Use along with [`NonFungibleEnumerable::total_supply()`] to enumerate
/// all the tokens in the contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `index` - Index of the token in the owner's local list.
///
/// # Notes
///
/// **IMPORTANT**: This function is only intended for non-sequential
/// `token_id`s. For sequential `token_id`s, no need to call a function,
/// the `token_id` itself acts as the global index.
///
/// # Errors
///
/// * [`NonFungibleTokenError::TokenNotFoundInGlobalList`] - When the token ID
///   is not found in the global enumeration.
pub fn get_token_id<T: NonFungibleEnumerable>(e: &Env, index: u32) -> u32 {
    let key = StorageKey::GlobalTokens(index);
    let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
        panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInGlobalList);
    };

    token_id
}

// ################## CHANGE STATE ##################

/// Creates a token with the next available `token_id` and assigns it to `to`.
/// Returns the `token_id` for the newly minted token.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new token.
///
/// # Errors
///
/// * refer to [`crate::mintable::mint`] errors.
/// * refer to [`increment_total_supply`] errors.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[token_id: u32]`
///
/// # Notes
///
/// This is a wrapper around [`crate::mintable::mint()`], that also
/// handles the storage updates for:
/// * total supply
pub fn sequential_mint<
    T: NonFungibleEnumerable<EnumerationStrategy = Sequential> + NonFungibleSequential,
>(
    e: &Env,
    to: &Address,
) -> u32 {
    let token_id = crate::mintable::sequential_mint::<T>(e, to);

    add_to_owner_enumeration::<T>(e, to, token_id);

    // We don't need the total supply, we just need to increment it.
    let _ = increment_total_supply::<T>(e);

    // We don't need to update the global lists, `token_id`s act as the global index
    // in sequential minting.

    token_id
}

/// Creates a token with the provided `token_id` and assigns it to `to`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address receiving the new token.
/// * `token_id` - Identifier for the new token.
///
/// # Errors
///
/// * refer to [`crate::storage::update`] errors.
/// * refer to [`increment_total_supply`] errors.
///
/// # Events
///
/// * topics - `["mint", to: Address]`
/// * data - `[token_id: u32]`
///
/// This is a wrapper around [`crate::storage::update()`], that also
/// handles the storage updates for:
/// * total supply
/// * owner_tokens enumeration
/// * global_tokens enumeration
pub fn non_sequential_mint<T: NonFungibleEnumerable<EnumerationStrategy = NonSequential>>(
    e: &Env,
    to: &Address,
    token_id: u32,
) {
    crate::mintable::mint::<T>(e, to, token_id);

    add_to_owner_enumeration::<T>(e, to, token_id);

    let total_supply = increment_total_supply::<T>(e);

    add_to_global_enumeration(e, token_id, total_supply);
}

/// Destroys the `token_id` from `account`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The account whose token is destroyed.
/// * `token_id` - The token to burn.
///
/// # Errors
///
/// * refer to [`crate::burnable::burn`] errors.
/// * refer to [`remove_from_owner_enumeration`] errors.
/// * refer to [`remove_from_global_enumeration`] errors.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: u32]`
///
/// # Notes
///
/// This is a wrapper around [`crate::burnable::burn()`], that also
/// handles the storage updates for:
/// * total supply
/// * owner_tokens enumeration
/// * global_tokens enumeration
pub fn burn<T: NonFungibleEnumerable>(e: &Env, from: &Address, token_id: u32) {
    crate::burnable::burn::<T>(e, from, token_id);
    T::EnumerationStrategy::remove(e, from, token_id);
}

pub fn burn_from<T: NonFungibleEnumerable>(
    e: &Env,
    spender: &Address,
    from: &Address,
    token_id: u32,
) {
    crate::burnable::burn_from::<T>(e, spender, from, token_id);
    T::EnumerationStrategy::remove(e, from, token_id);
}

// ################## LOW-LEVEL HELPERS ##################

/// Returns the old total supply (before the increment).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`NonFungibleTokenError::TokenIDsAreDepleted`] - When attempting to mint a
///   new token ID, but all token IDs are already in use.
pub fn increment_total_supply<T: NonFungibleEnumerable>(e: &Env) -> u32 {
    let total_supply = total_supply::<T>(e);
    let Some(new_total_supply) = total_supply.checked_add(1) else {
        panic_with_error!(e, NonFungibleTokenError::TokenIDsAreDepleted);
    };
    e.storage().instance().set(&StorageKey::TotalSupply, &new_total_supply);

    total_supply
}

/// Returns the new total supply (after the decrement).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn decrement_total_supply<T: NonFungibleEnumerable>(e: &Env) -> u32 {
    let total_supply = total_supply::<T>(e);
    let new_total_supply = total_supply.checked_sub(1).expect("Total supply cannot be negative");
    e.storage().instance().set(&StorageKey::TotalSupply, &new_total_supply);

    new_total_supply
}

/// Adds a token ID to the owner's enumeration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address of the owner.
/// * `token_id` - The token ID to add.
pub fn add_to_owner_enumeration<T: NonFungibleEnumerable>(e: &Env, owner: &Address, token_id: u32) {
    // we decrease 1 from the balance, because this function is called after balance
    // is manipulated (mint, transfer, etc.)
    let owner_balance = balance::<T>(e, owner) - 1;
    e.storage().persistent().set(
        &StorageKey::OwnerTokens(OwnerTokensKey { owner: owner.clone(), index: owner_balance }),
        &token_id,
    );
    e.storage().persistent().set(&StorageKey::OwnerTokensIndex(token_id), &owner_balance);
}

/// Removes a token ID from the owner's enumeration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The address of the owner.
/// * `to_be_removed_id` - The token ID to remove.
///
/// # Errors
///
/// * [`NonFungibleTokenError::TokenNotFoundInOwnerList`] - When the token ID is
///   not found in the owner's enumeration.
pub fn remove_from_owner_enumeration<T: NonFungibleEnumerable>(
    e: &Env,
    owner: &Address,
    to_be_removed_id: u32,
) {
    let Some(to_be_removed_index) =
        e.storage().persistent().get(&StorageKey::OwnerTokensIndex(to_be_removed_id))
    else {
        panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInOwnerList);
    };

    // This function is called after balance is manipulated, so do not need to
    // decrease 1 from the balance (burn, transfer, etc.).
    let last_token_index = balance::<T>(e, owner);

    // Update the `OwnerTokens`.
    if to_be_removed_index != last_token_index {
        // Before swap: [A, B, C, D]  (burning `B`, which is at index 1)
        // After swap:  [A, D, C, D]  (`D` moves to index 1, note that `B` isn't moved)
        // After deletion: [A, D, C]  (last item is deleted, effectively removing `B`)
        let last_token_id = get_owner_token_id::<T>(e, owner, last_token_index);
        e.storage().persistent().set(
            &StorageKey::OwnerTokens(OwnerTokensKey {
                owner: owner.clone(),
                index: to_be_removed_index,
            }),
            &last_token_id,
        );

        // Update the moved token's index.
        e.storage()
            .persistent()
            .set(&StorageKey::OwnerTokensIndex(last_token_id), &to_be_removed_index);
    }

    // Delete the last token from owner's local list.
    e.storage().persistent().remove(&StorageKey::OwnerTokens(OwnerTokensKey {
        owner: owner.clone(),
        index: last_token_index,
    }));
    e.storage().persistent().remove(&StorageKey::OwnerTokensIndex(to_be_removed_id));
}

/// Adds a token ID to the global enumeration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - The token ID to add.
/// * `total_supply` - The current total supply, acts as the index.
pub fn add_to_global_enumeration(e: &Env, token_id: u32, total_supply: u32) {
    e.storage().persistent().set(&StorageKey::GlobalTokens(total_supply), &token_id);
    e.storage().persistent().set(&StorageKey::GlobalTokensIndex(token_id), &total_supply);
}

/// Removes a token ID from the global enumeration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to_be_removed_id` - The token ID to remove.
/// * `last_token_index` - The index of the last token in the global
///   enumeration.
///
/// # Errors
///
/// * [`NonFungibleTokenError::TokenNotFoundInGlobalList`] - When the token ID
///   is not found in the global enumeration.
pub fn remove_from_global_enumeration<T: NonFungibleEnumerable>(
    e: &Env,
    to_be_removed_id: u32,
    last_token_index: u32,
) {
    let Some(to_be_removed_index) =
        e.storage().persistent().get::<_, u32>(&StorageKey::GlobalTokensIndex(to_be_removed_id))
    else {
        panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInGlobalList);
    };

    // unlike `remove_from_owner_enumeration`, we perform the swap without
    // checking if it's already the last token_id to avoid extra gas cost (being
    // last item in the global list is far less likely)

    // Before swap: [A, B, C, D]  (burning `B`, which is at index 1)
    // After swap:  [A, D, C, D]  (`D` moves to index 1, note that `B` isn't moved)
    // After deletion: [A, D, C]  (last item is deleted, effectively removing `B`)
    let last_token_id = get_token_id::<T>(e, last_token_index);
    e.storage().persistent().set(&StorageKey::GlobalTokens(to_be_removed_index), &last_token_id);

    // Update the moved token's index.
    e.storage()
        .persistent()
        .set(&StorageKey::GlobalTokensIndex(last_token_id), &to_be_removed_index);

    // Delete the last token from the global lists.
    e.storage().persistent().remove(&StorageKey::GlobalTokens(last_token_index));
    e.storage().persistent().remove(&StorageKey::GlobalTokensIndex(to_be_removed_id));
}
