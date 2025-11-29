use soroban_sdk::{contracttype, panic_with_error, Address, Env};

#[cfg(not(feature = "certora"))]
use crate::non_fungible::{
    emit_mint,
};

use crate::non_fungible::{
    Base, ContractOverrides, NonFungibleTokenError, OWNER_EXTEND_AMOUNT,
    OWNER_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT, TOKEN_TTL_THRESHOLD,
};

pub struct Enumerable;

impl ContractOverrides for Enumerable {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Enumerable::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Enumerable::transfer_from(e, spender, from, to, token_id);
    }
}

#[contracttype]
pub struct OwnerTokensKey {
    pub owner: Address,
    pub index: u32,
}

/// Storage keys for the data associated with the enumerable extension of
/// `NonFungibleToken`
#[contracttype]
pub enum NFTEnumerableStorageKey {
    TotalSupply,
    OwnerTokens(OwnerTokensKey),
    OwnerTokensIndex(/* token_id */ u32),
    GlobalTokens(/* index */ u32),
    GlobalTokensIndex(/* token_id */ u32),
}

impl Enumerable {
    // ################## QUERY STATE ##################

    /// Returns the total amount of tokens stored by the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn total_supply(e: &Env) -> u32 {
        e.storage().instance().get(&NFTEnumerableStorageKey::TotalSupply).unwrap_or(0)
    }

    /// Returns the `token_id` owned by `owner` at a given `index` in the
    /// owner's local list. Use along with [`Base::balance`] to enumerate all of
    /// `owner`'s tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - Account of the token's owner.
    /// * `index` - Index of the token in the owner's local list.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::TokenNotFoundInOwnerList`] - When the token
    ///   ID is not found in the owner's enumeration.
    pub fn get_owner_token_id(e: &Env, owner: &Address, index: u32) -> u32 {
        let key =
            NFTEnumerableStorageKey::OwnerTokens(OwnerTokensKey { owner: owner.clone(), index });
        let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
            panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInOwnerList);
        };
        e.storage().persistent().extend_ttl(&key, OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT);

        token_id
    }

    /// Returns the `token_id` at a given `index` in the global token list.
    /// Use along with [`Enumerable::total_supply`] to enumerate all the tokens
    /// in the contract.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `index` - Index of the token in the owner's local list.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::TokenNotFoundInGlobalList`] - When the token
    ///   ID is not found in the global enumeration.
    pub fn get_token_id(e: &Env, index: u32) -> u32 {
        let key = NFTEnumerableStorageKey::GlobalTokens(index);
        let Some(token_id) = e.storage().persistent().get::<_, u32>(&key) else {
            panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInGlobalList);
        };
        e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);

        token_id
    }

    // ################## CHANGE STATE ##################

    /// Creates a token with the next available `token_id` and assigns it to
    /// `to`. Returns the `token_id` for the newly minted token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new token.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::sequential_mint`] errors.
    /// * refer to [`Enumerable::increment_total_supply`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// This is a wrapper around [`Base::sequential_mint()`], that
    /// also handles the storage updates for:
    /// * total supply
    /// * global token list
    /// * owner token list
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// minting operations. Failure to implement proper authorization could
    /// lead to security vulnerabilities and unauthorized token creation.
    ///
    /// You probably want to do something like this (pseudo-code):
    ///
    /// ```ignore
    /// let admin = read_administrator(e);
    /// admin.require_auth();
    /// ```
    ///
    /// **IMPORTANT**: This function utilizes
    /// [`crate::non_fungible::sequential::increment_token_id`] to determine
    /// the next `token_id`, but it does NOT check if that `token_id` is
    /// already in use. If the developer has other means of minting tokens
    /// and generating `token_id`s, they should ensure that the `token_id`
    /// is unique and not already in use.
    pub fn sequential_mint(e: &Env, to: &Address) -> u32 {
        let token_id = Base::sequential_mint(e, to);

        Enumerable::add_to_enumerations(e, to, token_id);

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
    /// * refer to [`Base::update`] errors.
    /// * refer to [`Enumerable::increment_total_supply`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// This is a wrapper around [`Base::update()`], that also
    /// handles the storage updates for:
    /// * total supply
    /// * owner_tokens enumeration
    /// * global_tokens enumeration
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///Base
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// minting operations. Failure to implement proper authorization could
    /// lead to security vulnerabilities and unauthorized token creation.
    ///
    /// You probably want to do something like this (pseudo-code):
    ///
    /// ```ignore
    /// let admin = read_administrator(e);
    /// admin.require_auth();
    /// ```
    ///
    /// **IMPORTANT**: This function does NOT verify whether the provided
    /// `token_id` already exists. It is the developer's responsibility to
    /// ensure `token_id` uniqueness before passing it to this function. The
    /// strategy for generating `token_id`s varies by project and must be
    /// implemented accordingly.
    pub fn non_sequential_mint(e: &Env, to: &Address, token_id: u32) {
        Base::update(e, None, Some(to), token_id);
        #[cfg(not(feature = "certora"))]
        emit_mint(e, to, token_id);

        Enumerable::add_to_enumerations(e, to, token_id);
    }

    /// Destroys the token with `token_id` from `from`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::burn`] errors.
    /// * refer to [`Enumerable::remove_from_owner_enumeration`] errors.
    /// * refer to [`Enumerable::remove_from_global_enumeration`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// This is a wrapper around [`Base::burn`], that also
    /// handles the storage updates for:
    /// * total supply
    /// * owner_tokens enumeration
    /// * global_tokens enumeration
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        Base::burn(e, from, token_id);

        Enumerable::remove_from_enumerations(e, from, token_id);
    }

    /// Destroys the token with `token_id` from `from`, by using `spender`s
    /// approval.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::burn_from`] errors.
    /// * refer to [`Enumerable::remove_from_owner_enumeration`] errors.
    /// * refer to [`Enumerable::remove_from_global_enumeration`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// This is a wrapper around [`Base::burn_from`], that also
    /// handles the storage updates for:
    /// * total supply
    /// * owner_tokens enumeration
    /// * global_tokens enumeration
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        Base::burn_from(e, spender, from, token_id);

        Enumerable::remove_from_enumerations(e, from, token_id);
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
    /// * refer to [`Base::transfer`] errors.
    /// * refer to [`Enumerable::remove_from_owner_enumeration`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// This is a wrapper around [`Base::transfer`], that also
    /// handles the storage updates for:
    /// * owner_tokens enumeration
    pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Base::transfer(e, from, to, token_id);

        if from != to {
            Enumerable::remove_from_owner_enumeration(e, from, token_id);
            Enumerable::add_to_owner_enumeration(e, to, token_id);
        }
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
    /// * refer to [`Base::transfer_from`] errors.
    /// * refer to [`Enumerable::remove_from_owner_enumeration`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// This is a wrapper around [`Base::transfer_from`], that also
    /// handles the storage updates for:
    /// * owner_tokens enumeration
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Base::transfer_from(e, spender, from, to, token_id);

        if from != to {
            Enumerable::remove_from_owner_enumeration(e, from, token_id);
            Enumerable::add_to_owner_enumeration(e, to, token_id);
        }
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
    /// * [`NonFungibleTokenError::TokenIDsAreDepleted`] - When attempting to
    ///   mint a new token ID, but all token IDs are already in use.
    pub fn increment_total_supply(e: &Env) -> u32 {
        let total_supply = Enumerable::total_supply(e);
        let Some(new_total_supply) = total_supply.checked_add(1) else {
            panic_with_error!(e, NonFungibleTokenError::TokenIDsAreDepleted);
        };
        e.storage().instance().set(&NFTEnumerableStorageKey::TotalSupply, &new_total_supply);

        total_supply
    }

    /// Returns the new total supply (after the decrement).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::MathOverflow`] - If this function is called
    ///   when there are no tokens present.
    pub fn decrement_total_supply(e: &Env) -> u32 {
        let total_supply = Enumerable::total_supply(e);
        let Some(new_total_supply) = total_supply.checked_sub(1) else {
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().instance().set(&NFTEnumerableStorageKey::TotalSupply, &new_total_supply);

        new_total_supply
    }

    /// Adds a token to user's enumeration and global enumeration.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`Enumerable::add_to_owner_enumeration`] errors.
    /// * refer to [`Enumerable::increment_total_supply`] errors.
    pub fn add_to_enumerations(e: &Env, owner: &Address, token_id: u32) {
        Enumerable::add_to_owner_enumeration(e, owner, token_id);
        let total_supply = Enumerable::increment_total_supply(e);
        Enumerable::add_to_global_enumeration(e, token_id, total_supply);
    }

    /// Removes a token from user's enumeration and global enumeration.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address of the owner.
    /// * `token_id` - The token ID to remove.
    ///
    /// # Errors
    ///
    /// * refer to [`Enumerable::remove_from_owner_enumeration`] errors.
    /// * refer to [`Enumerable::decrement_total_supply`] errors.
    /// * refer to [`Enumerable::remove_from_global_enumeration`] errors.
    pub fn remove_from_enumerations(e: &Env, owner: &Address, token_id: u32) {
        Enumerable::remove_from_owner_enumeration(e, owner, token_id);
        let total_supply = Enumerable::decrement_total_supply(e);
        Enumerable::remove_from_global_enumeration(e, token_id, total_supply);
    }

    /// Adds a token ID to the owner's enumeration.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address of the owner.
    /// * `token_id` - The token ID to add.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::MathOverflow`] - When owner has no tokens,
    ///   and this function is called BEFORE the owner's balance is manipulated,
    ///   the indexing logic will underflow.
    ///
    /// # Notes
    ///
    /// This function is expected to be called after the balance of the owner
    /// is already manipulated (mint, transfer, etc.)
    pub fn add_to_owner_enumeration(e: &Env, owner: &Address, token_id: u32) {
        // balance is already incremented by 1, we need to subtract 1 from it
        // to get the `last_index + 1` (the index of the new token)
        let Some(owner_balance) = Base::balance(e, owner).checked_sub(1) else {
            panic_with_error!(e, NonFungibleTokenError::MathOverflow);
        };
        e.storage().persistent().set(
            &NFTEnumerableStorageKey::OwnerTokens(OwnerTokensKey {
                owner: owner.clone(),
                index: owner_balance,
            }),
            &token_id,
        );
        e.storage()
            .persistent()
            .set(&NFTEnumerableStorageKey::OwnerTokensIndex(token_id), &owner_balance);
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
    /// * [`NonFungibleTokenError::TokenNotFoundInOwnerList`] - When the token
    ///   ID is not found in the owner's enumeration.
    ///
    /// # Notes
    ///
    /// This function is expected to be called after the balance of the owner
    /// is already manipulated (mint, transfer, etc.)
    pub fn remove_from_owner_enumeration(e: &Env, owner: &Address, to_be_removed_id: u32) {
        let key = NFTEnumerableStorageKey::OwnerTokensIndex(to_be_removed_id);
        let Some(to_be_removed_index) = e.storage().persistent().get(&key) else {
            panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInOwnerList);
        };
        e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);

        // owner's balance is already decremented by 1, so it will be the index of the
        // last token in the enumeration list.
        let last_token_index = Base::balance(e, owner);

        // Update the `OwnerTokens`.
        if to_be_removed_index != last_token_index {
            // Before swap: [A, B, C, D]  (burning `B`, which is at index 1)
            // After swap:  [A, D, C, D]  (`D` moves to index 1, note that `B` isn't moved)
            // After deletion: [A, D, C]  (last item is deleted, effectively removing `B`)
            let last_token_id = Enumerable::get_owner_token_id(e, owner, last_token_index);
            e.storage().persistent().set(
                &NFTEnumerableStorageKey::OwnerTokens(OwnerTokensKey {
                    owner: owner.clone(),
                    index: to_be_removed_index,
                }),
                &last_token_id,
            );

            // Update the moved token's index.
            e.storage().persistent().set(
                &NFTEnumerableStorageKey::OwnerTokensIndex(last_token_id),
                &to_be_removed_index,
            );
        }

        // Delete the last token from owner's local list.
        e.storage().persistent().remove(&NFTEnumerableStorageKey::OwnerTokens(OwnerTokensKey {
            owner: owner.clone(),
            index: last_token_index,
        }));
        e.storage()
            .persistent()
            .remove(&NFTEnumerableStorageKey::OwnerTokensIndex(to_be_removed_id));
    }

    /// Adds a token ID to the global enumeration.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The token ID to add.
    /// * `total_supply` - The current total supply, acts as the index.
    pub fn add_to_global_enumeration(e: &Env, token_id: u32, total_supply: u32) {
        e.storage()
            .persistent()
            .set(&NFTEnumerableStorageKey::GlobalTokens(total_supply), &token_id);
        e.storage()
            .persistent()
            .set(&NFTEnumerableStorageKey::GlobalTokensIndex(token_id), &total_supply);
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
    /// * [`NonFungibleTokenError::TokenNotFoundInGlobalList`] - When the token
    ///   ID is not found in the global enumeration.
    pub fn remove_from_global_enumeration(e: &Env, to_be_removed_id: u32, last_token_index: u32) {
        let key = NFTEnumerableStorageKey::GlobalTokensIndex(to_be_removed_id);
        let Some(to_be_removed_index) = e.storage().persistent().get::<_, u32>(&key) else {
            panic_with_error!(e, NonFungibleTokenError::TokenNotFoundInGlobalList);
        };
        e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);

        // unlike `remove_from_owner_enumeration`, we perform the swap without
        // checking if it's already the last token_id to avoid extra gas cost (being
        // last item in the global list is far less likely)

        // Before swap: [A, B, C, D]  (burning `B`, which is at index 1)
        // After swap:  [A, D, C, D]  (`D` moves to index 1, note that `B` isn't moved)
        // After deletion: [A, D, C]  (last item is deleted, effectively removing `B`)
        let last_token_id = Enumerable::get_token_id(e, last_token_index);
        e.storage()
            .persistent()
            .set(&NFTEnumerableStorageKey::GlobalTokens(to_be_removed_index), &last_token_id);

        // Update the moved token's index.
        e.storage()
            .persistent()
            .set(&NFTEnumerableStorageKey::GlobalTokensIndex(last_token_id), &to_be_removed_index);

        // Delete the last token from the global lists.
        e.storage().persistent().remove(&NFTEnumerableStorageKey::GlobalTokens(last_token_index));
        e.storage()
            .persistent()
            .remove(&NFTEnumerableStorageKey::GlobalTokensIndex(to_be_removed_id));
    }
}
