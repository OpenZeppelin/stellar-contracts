use soroban_sdk::{contracttype, panic_with_error, Address, Env, String};
use stellar_constants::{
    OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT, TOKEN_TTL_THRESHOLD,
};

use crate::{
    burnable::emit_burn,
    emit_transfer,
    extensions::consecutive::emit_consecutive_mint,
    sequential::{self as sequential},
    Balance, Base, ContractOverrides, NonFungibleTokenError, TokenId,
};

pub struct Consecutive;

impl ContractOverrides for Consecutive {
    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Consecutive::owner_of(e, token_id)
    }

    fn token_uri(e: &Env, token_id: TokenId) -> String {
        Consecutive::token_uri(e, token_id)
    }

    fn transfer(e: &Env, from: &Address, to: &Address, token_id: TokenId) {
        Consecutive::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: TokenId) {
        Consecutive::transfer_from(e, spender, from, to, token_id);
    }

    fn approve(
        e: &Env,
        approver: &Address,
        approved: &Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Consecutive::approve(e, approver, approved, token_id, live_until_ledger);
    }
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    Approval(TokenId),
    Owner(TokenId),
    BurnedToken(TokenId),
}

impl Consecutive {
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
    pub fn owner_of(e: &Env, token_id: TokenId) -> Address {
        let max = sequential::next_token_id(e);
        let key = StorageKey::BurnedToken(token_id);
        let is_burned = e.storage().persistent().get(&key).unwrap_or(false);
        if is_burned {
            e.storage().persistent().extend_ttl(&key, TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT);
        }

        if token_id >= max || is_burned {
            panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
        }

        (0..=token_id)
            .rev()
            .map(StorageKey::Owner)
            // after the Protocol 23 upgrade, storage read cost is marginal,
            // making the consecutive storage reads justifiable
            .find_map(|key| {
                e.storage().persistent().get::<_, Address>(&key).inspect(|_| {
                    e.storage().persistent().extend_ttl(
                        &key,
                        OWNER_TTL_THRESHOLD,
                        OWNER_EXTEND_AMOUNT,
                    );
                })
            })
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
    pub fn token_uri(e: &Env, token_id: TokenId) -> String {
        let _ = Consecutive::owner_of(e, token_id);
        let base_uri = Base::base_uri(e);
        Base::compose_uri_for_token(e, base_uri, token_id)
    }

    // ################## CHANGE STATE ##################

    /// Mints a batch of tokens with consecutive ids and attributes them to
    /// `to`. This function does NOT handle authorization.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the recipient.
    /// * `amount` - The number of tokens to mint.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::increase_balance`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["consecutive_mint", to: Address]`
    /// * data - `[from_token_id: TokenId, to_token_id: TokenId]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The function intentionally lacks authorization controls.
    /// You MUST invoke it only from the constructor or implement proper
    /// authorization in the calling function. For example:
    ///
    /// ```ignore,rust
    /// fn mint_batch(e: &Env, to: &Address, amount: TokenId) {
    ///     // 1. Verify admin has minting privileges (optional)
    ///     let admin = e.storage().instance().get(&ADMIN_KEY).unwrap();
    ///     admin.require_auth();
    ///
    ///     // 2. Only then call the actual mint function
    ///     Consecutive::batch_mint(e, &to, amount);
    /// }
    /// ```
    ///
    /// Failing to add proper authorization could allow anyone to mint tokens!
    pub fn batch_mint(e: &Env, to: &Address, amount: Balance) -> TokenId {
        let first_id = sequential::increment_token_id(e, amount);

        e.storage().persistent().set(&StorageKey::Owner(first_id), &to);

        Base::increase_balance(e, to, amount);

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
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: TokenId]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn burn(e: &Env, from: &Address, token_id: TokenId) {
        from.require_auth();

        Consecutive::update(e, Some(from), None, token_id);
        emit_burn(e, from, token_id);
    }

    /// Destroys the `token_id` from `account`, ensuring ownership
    /// and approval checks, and emits a `burn` event.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::check_spender_approval`] errors.
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: TokenId]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: TokenId) {
        spender.require_auth();

        Base::check_spender_approval(e, spender, from, token_id);

        Consecutive::update(e, Some(from), None, token_id);
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
    /// * refer to [`Consecutive::update`] errors.
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

        Consecutive::update(e, Some(from), Some(to), token_id);
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
    /// * refer to [`Base::check_spender_approval`] errors.
    /// * refer to [`Consecutive::update`] errors.
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
    pub fn transfer_from(
        e: &Env,
        spender: &Address,
        from: &Address,
        to: &Address,
        token_id: TokenId,
    ) {
        spender.require_auth();

        Base::check_spender_approval(e, spender, from, token_id);

        Consecutive::update(e, Some(from), Some(to), token_id);
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
    /// * refer to [`Consecutive::owner_of`] errors.
    /// * refer to [`Base::approve_for_owner`] errors.
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

        let owner = Consecutive::owner_of(e, token_id);
        Base::approve_for_owner(e, &owner, approver, approved, token_id, live_until_ledger);
    }

    /// Low-level function for handling transfers, mints and burns of an NFT,
    /// without handling authorization. Updates ownership records, adjusts
    /// balances, and clears existing approvals.
    ///
    /// The difference with [`Base::update`] is that the
    /// current function:
    /// 1. explicitly adds burned tokens to storage in
    ///    `StorageKey::BurnedToken`,
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
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the `from` address is
    ///   not the owner of the token.
    /// * refer to [`owner_of`] errors.
    /// * refer to [`decrease_balance`] errors.
    /// * refer to [`increase_balance`] errors.
    pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, token_id: TokenId) {
        if let Some(from_address) = from {
            let owner = Consecutive::owner_of(e, token_id);

            // Ensure the `from` address is indeed the owner.
            if owner != *from_address {
                panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
            }

            Base::decrease_balance(e, from_address, 1);

            // Clear any existing approval
            let approval_key = StorageKey::Approval(token_id);
            e.storage().temporary().remove(&approval_key);

            // Set the next token to prev owner
            Consecutive::set_owner_for(e, from_address, token_id + 1);
        } else {
            // nothing to do for the `None` case, since we don't track
            // `total_supply`
        }

        if let Some(to_address) = to {
            Base::increase_balance(e, to_address, 1);

            // Set the new owner
            e.storage().persistent().set(&StorageKey::Owner(token_id), to_address);
        } else {
            // Burning: `to` is None
            e.storage().persistent().remove(&StorageKey::Owner(token_id));

            e.storage().persistent().set(&StorageKey::BurnedToken(token_id), &true);
        }
    }

    /// Low-level function that sets owner of `token_id` to `to`, without
    /// handling authorization. The function does not panic and sets the
    /// owner only if:
    /// - the token exists and
    /// - the token has not been burned and
    /// - the token doesn't have an owner.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `to` - The owner's address.
    /// * `token_id` - The identifier of the token being set.
    pub fn set_owner_for(e: &Env, to: &Address, token_id: TokenId) {
        let max = sequential::next_token_id(e);
        let owner_key = StorageKey::Owner(token_id);
        let has_owner = e.storage().persistent().has(&owner_key);
        if has_owner {
            e.storage().persistent().extend_ttl(
                &owner_key,
                OWNER_TTL_THRESHOLD,
                OWNER_EXTEND_AMOUNT,
            );
        }

        let burned_token_key = StorageKey::BurnedToken(token_id);
        let is_burned = e.storage().persistent().get(&burned_token_key).unwrap_or(false);
        if is_burned {
            e.storage().persistent().extend_ttl(
                &burned_token_key,
                TOKEN_TTL_THRESHOLD,
                TOKEN_EXTEND_AMOUNT,
            );
        }

        if token_id < max && !has_owner && !is_burned {
            e.storage().persistent().set(&StorageKey::Owner(token_id), to);
        }
    }
}
