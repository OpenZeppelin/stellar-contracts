use soroban_sdk::{contracttype, panic_with_error, Address, Env, String};

use super::emit_consecutive_mint;
use crate::{
    burnable::emit_burn,
    emit_transfer,
    sequential::{self as sequential},
    storage::{approve_for_owner, check_spender_approval, decrease_balance, increase_balance},
    NonFungibleTokenError, TokenId,
};

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Approval(TokenId),
    Owner(TokenId),
    BurnedToken(TokenId),
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
pub fn consecutive_owner_of(e: &Env, token_id: TokenId) -> Address {
    let max = sequential::next_token_id(e);
    let is_burned =
        e.storage().persistent().get(&StorageKey::BurnedToken(token_id)).unwrap_or(false);

    if token_id >= max || is_burned {
        panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
    }

    (0..=token_id)
        .rev()
        .map(StorageKey::Owner)
        // after the Protocol 23 upgrade, storage read cost is marginal,
        // making the consecutive storage reads justifiable
        .find_map(|key| e.storage().persistent().get::<_, Address>(&key))
        .unwrap_or_else(|| panic_with_error!(&e, NonFungibleTokenError::NonExistentToken))
}

/// Returns the URI for a specific `token_id`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token_id` - The identifier of the token.
///
/// # Errors
///
/// * refer to [`owner_of`] errors.
/// * refer to [`base_uri`] errors.
pub fn consecutive_token_uri(e: &Env, token_id: TokenId) -> String {
    let _ = consecutive_owner_of(e, token_id);
    let base_uri = crate::base_uri(e);
    crate::storage::compose_uri_for_token(e, base_uri, token_id)
}

// ################## CHANGE STATE ##################

/// Mints a batch of tokens with consecutive ids and attributes them to `to`.
/// This function does NOT handle authorization.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `to` - The address of the recipient.
/// * `amount` - The number of tokens to mint.
///
/// # Errors
///
/// * refer to [`crate::storage::increase_balance`] errors.
///
/// # Events
///
/// * topics - `["consecutive_mint", to: Address]`
/// * data - `[from_token_id: TokenId, to_token_id: TokenId]`
///
/// # Security Warning
///
/// **IMPORTANT**: The function intentionally lacks authorization controls. You
/// MUST invoke it only from the constructor or implement proper authorization
/// in the calling function. For example:
///
/// ```ignore,rust
/// fn mint_batch(e: &Env, to: &Address, amount: TokenId) {
///     // 1. Verify admin has minting privileges (optional)
///     let admin = e.storage().instance().get(&ADMIN_KEY).unwrap();
///     admin.require_auth();
///
///     // 2. Only then call the actual mint function
///     crate::consecutive::batch_mint(e, &to, amount);
/// }
/// ```
///
/// Failing to add proper authorization could allow anyone to mint tokens!
pub fn consecutive_batch_mint(e: &Env, to: &Address, amount: TokenId) -> TokenId {
    let first_id = sequential::increment_token_id(e, amount);

    e.storage().persistent().set(&StorageKey::Owner(first_id), &to);

    increase_balance(e, to, amount);

    let last_id = first_id + amount - 1;
    emit_consecutive_mint(e, to, first_id, last_id);

    // return the last minted id
    last_id
}

/// Destroys the `token_id` from `account`, ensuring ownership
/// checks, and emits a `burn` event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The account whose token is destroyed.
/// * `token_id` - The token to burn.
///
/// # Errors
///
/// * refer to [`self::consecutive_update`] errors.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: TokenId]`
///
/// # Notes
///
/// Authorization for `from` is required.
pub fn consecutive_burn(e: &Env, from: &Address, token_id: TokenId) {
    from.require_auth();

    self::consecutive_update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);
}

/// Destroys the `token_id` from `account`, ensuring ownership
/// and approval checks, and emits a `burn` event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The account that is allowed to burn the token on behalf of the
///   owner.
/// * `from` - The account whose token is destroyed.
/// * `token_id` - The token to burn.
///
/// # Errors
///
/// * refer to [`crate::storage::check_spender_approval`] errors.
/// * refer to [`self::consecutive_update`] errors.
///
/// # Events
///
/// * topics - `["burn", from: Address]`
/// * data - `[token_id: TokenId]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn consecutive_burn_from(e: &Env, spender: &Address, from: &Address, token_id: TokenId) {
    spender.require_auth();

    check_spender_approval(e, spender, from, token_id);

    self::consecutive_update(e, Some(from), None, token_id);
    emit_burn(e, from, token_id);
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
/// * refer to [`self::consecutive_update`] errors.
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
pub fn consecutive_transfer(e: &Env, from: &Address, to: &Address, token_id: TokenId) {
    from.require_auth();

    self::consecutive_update(e, Some(from), Some(to), token_id);
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
/// * refer to [`crate::storage::check_spender_approval`] errors.
/// * refer to [`self::consecutive_update`] errors.
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
pub fn consecutive_transfer_from(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    token_id: TokenId,
) {
    spender.require_auth();

    check_spender_approval(e, spender, from, token_id);

    self::consecutive_update(e, Some(from), Some(to), token_id);
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
/// * refer to [`self::consecutive_owner_of`] errors.
/// * refer to [`crate::storage::approve_for_owner`] errors.
///
/// # Events
///
/// * topics - `["approve", owner: Address, token_id: TokenId]`
/// * data - `[approved: Address, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for `approver` is required.
pub fn consecutive_approve(
    e: &Env,
    approver: &Address,
    approved: &Address,
    token_id: TokenId,
    live_until_ledger: u32,
) {
    approver.require_auth();

    let owner = consecutive_owner_of(e, token_id);
    approve_for_owner(e, &owner, approver, approved, token_id, live_until_ledger);
}

/// Low-level function for handling transfers, mints and burns of an NFT,
/// without handling authorization. Updates ownership records, adjusts balances,
/// and clears existing approvals.
///
/// The difference with [`crate::storage::consecutive_update`] is that the
/// current function:
/// 1. explicitly adds burned tokens to storage in `StorageKey::BurnedToken`,
/// 2. sets the next token (if any) to the previous owner.
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
/// * refer to [`consecutive_owner_of`] errors.
/// * refer to [`decrease_balance`] errors.
/// * refer to [`increase_balance`] errors.
pub fn consecutive_update(
    e: &Env,
    from: Option<&Address>,
    to: Option<&Address>,
    token_id: TokenId,
) {
    if let Some(from_address) = from {
        let owner = consecutive_owner_of(e, token_id);

        // Ensure the `from` address is indeed the owner.
        if owner != *from_address {
            panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
        }

        decrease_balance(e, from_address, 1);

        // Clear any existing approval
        let approval_key = StorageKey::Approval(token_id);
        e.storage().temporary().remove(&approval_key);

        // Set the next token to prev owner
        consecutive_set_owner_for(e, from_address, token_id + 1);
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

        e.storage().persistent().set(&StorageKey::BurnedToken(token_id), &true);
    }
}

/// Low-level function that sets owner of `token_id` to `to`, without handling
/// authorization. The function does not panic and sets the owner only if:
/// - the token exists and
/// - the token has not been burned and
/// - the token doesn't have an owner.
///
/// # Arguments
///
/// * `e` - The environment reference.
/// * `to` - The owner's address.
/// * `token_id` - The identifier of the token being set.
pub fn consecutive_set_owner_for(e: &Env, to: &Address, token_id: TokenId) {
    let max = sequential::next_token_id(e);
    let has_owner = e.storage().persistent().has(&StorageKey::Owner(token_id));
    let is_burned =
        e.storage().persistent().get(&StorageKey::BurnedToken(token_id)).unwrap_or(false);

    if token_id < max && !has_owner && !is_burned {
        e.storage().persistent().set(&StorageKey::Owner(token_id), to);
    }
}
