use soroban_sdk::{
    contracttype, panic_with_error, Address, Env, IntoVal, String, Symbol, Val, Vec,
};
use stellar_contract_utils::pausable::{paused, PausableError};

use crate::{
    fungible::{emit_transfer, Base, ContractOverrides, StorageKey},
    rwa::{
        emit_address_frozen, emit_burn, emit_claim_topics_and_issuers_added, emit_compliance_added,
        emit_identity_registry_storage_added, emit_identity_verifier_added, emit_mint,
        emit_recovery_success, emit_token_information_updated, emit_tokens_frozen,
        emit_tokens_unfrozen, RWAError, FROZEN_EXTEND_AMOUNT, FROZEN_TTL_THRESHOLD,
    },
};

/// Storage keys for the data associated with `RWA` token
#[contracttype]
pub enum RWAStorageKey {
    /// Frozen status of an address (true = frozen, false = not frozen)
    AddressFrozen(Address),
    /// Amount of tokens frozen for a specific address
    FrozenTokens(Address),
    /// Identity Verifier contract address
    IdentityVerifier,
    /// Compliance contract address
    Compliance,
    /// OnchainID contract address
    OnchainId,
    /// Version of the token
    Version,
    /// Claim Topics and Issuers contract address
    ClaimTopicsAndIssuers,
    /// Identity Registry Storage contract address
    IdentityRegistryStorage,
}

// TODO: change `invoke_contract` calls to `client` instead when `compliance`
// and `identity_verifier` is merged

pub struct RWA;

impl ContractOverrides for RWA {
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        RWA::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        RWA::transfer_from(e, spender, from, to, amount);
    }
}

impl RWA {
    // ################## QUERY STATE ##################

    /// Returns the token version.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::VersionNotSet`] - When the version is not set.
    pub fn version(e: &Env) -> String {
        e.storage()
            .instance()
            .get(&RWAStorageKey::Version)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::VersionNotSet))
    }

    /// Returns the address of the onchain ID of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::OnchainIdNotSet`] - When the onchain ID is not set.
    pub fn onchain_id(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::OnchainId)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::OnchainIdNotSet))
    }

    /// Returns the Identity Verifier linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not set.
    pub fn identity_verifier(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::IdentityVerifier)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityVerifierNotSet))
    }

    /// Returns the Compliance contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not
    ///   set.
    pub fn compliance(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::Compliance)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::ComplianceNotSet))
    }

    /// Returns the Claim Topics and Issuers contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ClaimTopicsAndIssuersNotSet`] - When the claim topics and
    ///   issuers contract is not set.
    pub fn claim_topics_and_issuers(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::ClaimTopicsAndIssuers)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::ClaimTopicsAndIssuersNotSet))
    }

    /// Returns the Identity Registry Storage contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityRegistryStorageNotSet`] - When the identity
    ///   registry storage contract is not set.
    pub fn identity_registry_storage(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::IdentityRegistryStorage)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityRegistryStorageNotSet))
    }

    /// Returns the freezing status of a wallet. Frozen wallets cannot send or
    /// receive funds.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet to check.
    pub fn is_frozen(e: &Env, user_address: &Address) -> bool {
        let key = RWAStorageKey::AddressFrozen(user_address.clone());
        if let Some(frozen) = e.storage().persistent().get::<_, bool>(&key) {
            e.storage().persistent().extend_ttl(&key, FROZEN_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT);
            frozen
        } else {
            false
        }
    }

    /// Returns the amount of tokens that are partially frozen on a wallet.
    /// The amount of frozen tokens is always <= to the total balance of the
    /// wallet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet on which get_frozen_tokens
    ///   is called.
    pub fn get_frozen_tokens(e: &Env, user_address: &Address) -> i128 {
        let key = RWAStorageKey::FrozenTokens(user_address.clone());
        if let Some(frozen_amount) = e.storage().persistent().get::<_, i128>(&key) {
            e.storage().persistent().extend_ttl(&key, FROZEN_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT);
            frozen_amount
        } else {
            0
        }
    }

    /// Returns the amount of free (unfrozen) tokens for an address.
    /// This is calculated as total balance minus frozen tokens.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address to check.
    pub fn get_free_tokens(e: &Env, user_address: &Address) -> i128 {
        let total_balance = Base::balance(e, user_address);
        let frozen_tokens = Self::get_frozen_tokens(e, user_address);

        // frozen tokens cannot be greater than total balance, necessary checks are done
        // in state changing functions
        total_balance - frozen_tokens
    }

    // ################## CHANGE STATE ##################

    /// Forced transfer of `amount` tokens from `from` to `to`.
    /// This function can unfreeze tokens if needed for regulatory compliance.
    /// It bypasses paused state and frozen address checks.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address holding the tokens.
    /// * `to` - The address receiving the tokens.
    /// * `amount` - The amount of tokens to transfer.
    ///
    /// # Errors
    ///
    /// * [`RWAError::InsufficientBalance`] - When attempting to transfer more
    ///   tokens than available.
    /// * refer to [`Base::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// This function bypasses freezing restrictions and can unfreeze tokens
    /// as needed. It's intended for regulatory compliance and recovery
    /// scenarios.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization and freezing checks.
    /// Should only be used by authorized compliance or admin functions.
    pub fn forced_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        let from_balance = Base::balance(e, from);
        if from_balance < amount {
            panic_with_error!(e, RWAError::InsufficientBalance);
        }

        // Check if we need to unfreeze tokens to complete the transfer
        let free_tokens = Self::get_free_tokens(e, from);
        if free_tokens < amount {
            let tokens_to_unfreeze = amount - free_tokens;
            let current_frozen = Self::get_frozen_tokens(e, from);
            let new_frozen = current_frozen - tokens_to_unfreeze;

            e.storage().persistent().set(&RWAStorageKey::FrozenTokens(from.clone()), &new_frozen);
            emit_tokens_unfrozen(e, from, tokens_to_unfreeze);
        }

        Base::update(e, Some(from), Some(to), amount);

        Self::trigger_compliance_hook(
            e,
            "transferred",
            Vec::from_array(e, [from.into_val(e), to.into_val(e), amount.into_val(e)]),
        );

        emit_transfer(e, from, to, amount);
    }

    /// Mints `amount` tokens to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Errors
    ///
    /// refer to [`RWA::verify_identity`] errors.
    /// refer to [`RWA::validate_compliance`] errors.
    /// refer to [`Base::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
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
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        // Verify identity verifier for the recipient address
        Self::verify_identity(e, to);

        Base::update(e, None, Some(to), amount);

        Self::validate_compliance(e, None, to, amount);

        Self::trigger_compliance_hook(
            e,
            "created",
            Vec::from_array(e, [to.into_val(e), amount.into_val(e)]),
        );

        emit_mint(e, to, amount);
    }

    /// Burns `amount` tokens from `user_address`. Updates the total supply
    /// accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address from which to burn tokens.
    /// * `amount` - The amount of tokens to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", user_address: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Notes
    ///
    /// Authorization for `user_address` is required.
    pub fn burn(e: &Env, user_address: &Address, amount: i128) {
        user_address.require_auth();

        // Check if we need to unfreeze tokens to complete the burn
        let free_tokens = Self::get_free_tokens(e, user_address);
        if free_tokens < amount {
            let tokens_to_unfreeze = amount - free_tokens;
            let current_frozen = Self::get_frozen_tokens(e, user_address);
            let new_frozen = current_frozen - tokens_to_unfreeze;

            e.storage()
                .persistent()
                .set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
            emit_tokens_unfrozen(e, user_address, tokens_to_unfreeze);
        }

        Base::update(e, Some(user_address), None, amount);

        Self::trigger_compliance_hook(
            e,
            "destroyed",
            Vec::from_array(e, [user_address.into_val(e), amount.into_val(e)]),
        );

        emit_burn(e, user_address, amount);
    }

    /// Recovery function used to force transfer tokens from a lost wallet to a
    /// new wallet. This function transfers all tokens and clears frozen
    /// status from the lost wallet. Returns `true` if recovery was
    /// successful, `false` if no tokens to recover.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `lost_wallet` - The address of the wallet that lost access.
    /// * `new_wallet` - The address of the new wallet to receive the tokens.
    /// * `investor_onchain_id` - The onchain ID of the investor for
    ///   verification.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not configured.
    /// * [`RWAError::AddressNotVerified`] - When the new wallet is not
    ///   verified.
    /// * [`RWAError::RecoveryFailed`] - When recovery parameters are invalid.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", lost_wallet: Address, new_wallet: Address]`
    /// * data - `[amount: i128]`
    /// * topics - `["recovery", lost_wallet: Address, new_wallet: Address,
    ///   investor_onchain_id: Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// This function automatically unfreezes all frozen tokens and clears the
    /// frozen status of the lost wallet before transferring.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization and compliance
    /// checks. Should only be used by authorized recovery or admin
    /// functions.
    pub fn recovery_address(
        e: &Env,
        lost_wallet: &Address,
        new_wallet: &Address,
        investor_onchain_id: &Address,
    ) -> bool {
        // Verify identity for the new wallet
        Self::verify_identity(e, new_wallet);

        let lost_balance = Base::balance(e, lost_wallet);
        if lost_balance == 0 {
            return false;
        }

        // Transfer all tokens from lost wallet to new wallet
        let frozen_tokens = Self::get_frozen_tokens(e, lost_wallet);

        // If there are frozen tokens, unfreeze them first
        if frozen_tokens > 0 {
            e.storage().persistent().set(&RWAStorageKey::FrozenTokens(lost_wallet.clone()), &0i128);
            emit_tokens_unfrozen(e, lost_wallet, frozen_tokens);
        }

        // Transfer all balance
        let new_balance = Base::balance(e, new_wallet) + lost_balance;
        e.storage().persistent().set(&StorageKey::Balance(lost_wallet.clone()), &0i128);
        e.storage().persistent().set(&StorageKey::Balance(new_wallet.clone()), &new_balance);

        // Clear frozen status if set
        if Self::is_frozen(e, lost_wallet) {
            e.storage()
                .persistent()
                .set(&RWAStorageKey::AddressFrozen(lost_wallet.clone()), &false);
        }

        emit_transfer(e, lost_wallet, new_wallet, lost_balance);
        emit_recovery_success(e, lost_wallet, new_wallet, investor_onchain_id);

        true
    }

    /// Sets the frozen status for an address. Frozen wallets cannot send or
    /// receive funds.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address to freeze or unfreeze.
    /// * `freeze` - `true` to freeze the address, `false` to unfreeze.
    ///
    /// # Events
    ///
    /// * topics - `["freeze", user_address: Address, freeze: bool, caller:
    ///   Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_address_frozen(e: &Env, caller: &Address, user_address: &Address, freeze: bool) {
        e.storage().persistent().set(&RWAStorageKey::AddressFrozen(user_address.clone()), &freeze);

        emit_address_frozen(e, user_address, freeze, caller);
    }

    /// Freezes a specified amount of tokens for a given address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to freeze tokens.
    /// * `amount` - The amount of tokens to freeze.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When `amount < 0`.
    /// * [`RWAError::InsufficientBalance`] - When trying to freeze more tokens
    ///   than available.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_frozen", user_address: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn freeze_partial_tokens(e: &Env, user_address: &Address, amount: i128) {
        if amount < 0 {
            panic_with_error!(e, RWAError::LessThanZero);
        }

        let current_balance = Base::balance(e, user_address);
        let current_frozen = Self::get_frozen_tokens(e, user_address);
        let new_frozen = current_frozen + amount;

        if new_frozen > current_balance {
            panic_with_error!(e, RWAError::InsufficientBalance);
        }

        e.storage()
            .persistent()
            .set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
        emit_tokens_frozen(e, user_address, amount);
    }

    /// Unfreezes a specified amount of tokens for a given address.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to unfreeze tokens.
    /// * `amount` - The amount of tokens to unfreeze.
    ///
    /// # Errors
    ///
    /// * [`RWAError::LessThanZero`] - When `amount < 0`.
    /// * [`RWAError::InsufficientFreeTokens`] - When trying to unfreeze more
    ///   tokens than are frozen.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_unfrozen", user_address: Address]`
    /// * data - `[amount: i128]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn unfreeze_partial_tokens(e: &Env, user_address: &Address, amount: i128) {
        if amount < 0 {
            panic_with_error!(e, RWAError::LessThanZero);
        }

        let current_frozen = Self::get_frozen_tokens(e, user_address);
        if current_frozen < amount {
            panic_with_error!(e, RWAError::InsufficientFreeTokens);
        }

        let new_frozen = current_frozen - amount;
        e.storage()
            .persistent()
            .set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
        emit_tokens_unfrozen(e, user_address, amount);
    }

    /// Sets the onchain ID of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `onchain_id` - The new onchain ID address for the token.
    ///
    /// # Events
    ///
    /// * topics - `["token_info", name: Symbol, symbol: Symbol, decimals: u32,
    ///   version: Symbol, onchain_id: Address]`
    /// * data - `[]`
    ///
    /// # Errors
    ///
    /// * refer to [`Base::get_metadata`] errors.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_onchain_id(e: &Env, onchain_id: &Address) {
        e.storage().instance().set(&RWAStorageKey::OnchainId, onchain_id);

        emit_token_information_updated(e, None, None, None, None, Some(onchain_id));
    }

    /// Sets the Identity Verifier for the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_verifier` - The address of the identity verifier contract.
    ///
    /// # Events
    ///
    /// * topics - `["id_reg_add", identity_verifier: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_identity_verifier(e: &Env, identity_verifier: &Address) {
        e.storage().instance().set(&RWAStorageKey::IdentityVerifier, identity_verifier);
        emit_identity_verifier_added(e, identity_verifier);
    }

    /// Sets the compliance contract of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The address of the compliance contract.
    ///
    /// # Events
    ///
    /// * topics - `["comp_add", compliance: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_compliance(e: &Env, compliance: &Address) {
        e.storage().instance().set(&RWAStorageKey::Compliance, compliance);
        emit_compliance_added(e, compliance);
    }

    /// Sets the claim topics and issuers contract of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topics_and_issuers` - The address of the claim topics and
    ///   issuers contract.
    ///
    /// # Events
    ///
    /// * topics - `["claim_topics_issuers_add", claim_topics_and_issuers:
    ///   Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: &Address) {
        e.storage().instance().set(&RWAStorageKey::ClaimTopicsAndIssuers, claim_topics_and_issuers);
        emit_claim_topics_and_issuers_added(e, claim_topics_and_issuers);
    }

    /// Sets the identity registry storage contract of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_registry_storage` - The address of the identity registry
    ///   storage contract.
    ///
    /// # Events
    ///
    /// * topics - `["identity_registry_storage_add", identity_registry_storage:
    ///   Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_identity_registry_storage(e: &Env, identity_registry_storage: &Address) {
        e.storage()
            .instance()
            .set(&RWAStorageKey::IdentityRegistryStorage, identity_registry_storage);
        emit_identity_registry_storage_added(e, identity_registry_storage);
    }

    // ########## HELPER FUNCTIONS FOR CONTRACT INTEGRATION ##########

    /// Verifies that an address is registered and verified in the identity
    /// verifier.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address to verify.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is
    ///   not configured.
    /// * [`RWAError::AddressNotVerified`] - When the address is not verified in
    ///   the identity verifier.
    ///
    /// # Notes
    ///
    /// This function calls the identity verifier contract to check if the
    /// address has a valid, verified identity. The identity verifier should
    /// implement a `is_verify` function that returns a boolean.
    fn verify_identity(e: &Env, user_address: &Address) {
        let identity_verifier_addr = Self::identity_verifier(e);

        // Call the identity verifier contract to verify the address
        let is_verified: bool = e.invoke_contract(
            &identity_verifier_addr,
            &Symbol::new(e, "is_verified"),
            Vec::from_array(e, [user_address.into_val(e)]),
        );

        if !is_verified {
            panic_with_error!(e, RWAError::AddressNotVerified);
        }
    }

    /// Validates compliance rules for a token transfer.
    /// Mint is also considered as a transfer, but burn is not.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address sending tokens.
    /// * `to` - The address receiving tokens.
    /// * `amount` - The amount of tokens being transferred.
    ///
    /// # Errors
    ///
    /// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not
    ///   configured.
    /// * [`RWAError::TransferNotCompliant`] - When the transfer violates
    ///   compliance rules.
    ///
    /// # Notes
    ///
    /// This function calls the compliance contract to validate the transfer
    /// against configured compliance rules. The compliance contract should
    /// implement a `can_transfer` function that returns a boolean.
    fn validate_compliance(e: &Env, from: Option<&Address>, to: &Address, amount: i128) {
        let compliance_addr = Self::compliance(e);

        let can_transfer: bool = e.invoke_contract(
            &compliance_addr,
            &Symbol::new(e, "can_transfer"),
            Vec::from_array(e, [from.into_val(e), to.into_val(e), amount.into_val(e)]),
        );

        if !can_transfer {
            panic_with_error!(e, RWAError::TransferNotCompliant);
        }
    }

    fn trigger_compliance_hook(e: &Env, hook_name: &str, arguments: Vec<Val>) {
        let compliance_addr = Self::compliance(e);
        e.invoke_contract::<()>(&compliance_addr, &Symbol::new(e, hook_name), arguments);
    }

    // ################## OVERRIDDEN FUNCTIONS ##################

    /// This is a wrapper around [`Base::update()`] to enable
    /// the compatibility across [`crate::fungible::FungibleToken`]
    /// with [`crate::rwa::RWAToken`]
    ///
    /// The main differences are:
    /// - checks for if the contract is paused
    /// - checks for if the addresses are frozen
    /// - checks for if the from address have enough free tokens (unfrozen
    ///   tokens)
    /// - enforces identity verification for both addresses
    /// - enforces compliance rules for the transfer
    /// - triggers `transferred` hook call from the compliance contract
    ///
    /// Please refer to [`Base::update`] for the inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        from.require_auth();

        // Check if contract is paused
        if paused(e) {
            panic_with_error!(e, PausableError::EnforcedPause);
        }

        // Check if addresses are frozen
        if Self::is_frozen(e, from) {
            panic_with_error!(e, RWAError::AddressFrozen);
        }
        if Self::is_frozen(e, to) {
            panic_with_error!(e, RWAError::AddressFrozen);
        }

        // Check if there are enough free tokens (not frozen)
        let free_tokens = Self::get_free_tokens(e, from);
        if free_tokens < amount {
            panic_with_error!(e, RWAError::InsufficientFreeTokens);
        }

        // Verify identity verifier for both addresses
        Self::verify_identity(e, from);
        Self::verify_identity(e, to);

        // Validate compliance rules for the transfer
        Self::validate_compliance(e, Some(from), to, amount);

        Base::update(e, Some(from), Some(to), amount);

        Self::trigger_compliance_hook(
            e,
            "transferred",
            Vec::from_array(e, [from.into_val(e), to.into_val(e), amount.into_val(e)]),
        );

        emit_transfer(e, from, to, amount);
    }

    /// This is a wrapper around [`Base::update()`] to enable
    /// the compatibility across [`crate::fungible::FungibleToken`]
    /// with [`crate::rwa::RWAToken`]
    ///
    /// Please refer to [`Base::update`] and [`Self::transfer`] for the inline
    /// documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Base::spend_allowance(e, from, spender, amount);
        Self::transfer(e, from, to, amount);
    }
}
