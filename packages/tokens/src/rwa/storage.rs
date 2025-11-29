use soroban_sdk::{contracttype, panic_with_error, Address, Env, String};
use stellar_contract_utils::pausable::{paused, PausableError};

use crate::{
    fungible::{Base, ContractOverrides},
    rwa::{
        compliance::ComplianceClient,
        identity_verifier::IdentityVerifierClient, RWAError, FROZEN_EXTEND_AMOUNT,
        FROZEN_TTL_THRESHOLD,
    },
};

#[cfg(not(feature = "certora"))]
use crate::{
    fungible::{emit_transfer,},
    rwa::{
        emit_address_frozen, emit_burn, emit_compliance_set,
        emit_identity_verifier_set, emit_mint, emit_recovery_success,
        emit_token_onchain_id_updated, emit_tokens_frozen, emit_tokens_unfrozen,
    },
};

/// Storage keys for the data associated with `RWA` token
#[contracttype]
pub enum RWAStorageKey {
    /// Frozen status of an address (true = frozen, false = not frozen)
    AddressFrozen(Address),
    /// Amount of tokens frozen for a specific address
    FrozenTokens(Address),
    /// Compliance contract address
    Compliance,
    /// OnchainID contract address
    OnchainId,
    /// Version of the token
    Version,
    /// Identity Verifier contract address
    IdentityVerifier,
}

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

    /// Returns the Identity Verifier contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier
    ///   contract is not set.
    pub fn identity_verifier(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&RWAStorageKey::IdentityVerifier)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityVerifierNotSet))
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
            #[cfg(not(feature = "certora"))]
            emit_tokens_unfrozen(e, from, tokens_to_unfreeze);
        }

        Base::update(e, Some(from), Some(to), amount);

        let compliance_addr = Self::compliance(e);
        let compliance_client = ComplianceClient::new(e, &compliance_addr);
        compliance_client.transferred(from, to, &amount, &e.current_contract_address());
        #[cfg(not(feature = "certora"))]
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
    /// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not
    ///   configured.
    /// * [`RWAError::MintNotCompliant`] - When the mint operation violates
    ///   compliance rules.
    /// * refer to [`Base::update`] errors.
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
        let identity_verifier_addr = Self::identity_verifier(e);
        let identity_verifier_client = IdentityVerifierClient::new(e, &identity_verifier_addr);
        identity_verifier_client.verify_identity(to);

        let compliance_addr = Self::compliance(e);
        let compliance_client = ComplianceClient::new(e, &compliance_addr);

        let can_create: bool =
            compliance_client.can_create(to, &amount, &e.current_contract_address());

        if !can_create {
            panic_with_error!(e, RWAError::MintNotCompliant);
        }

        Base::update(e, None, Some(to), amount);

        compliance_client.created(to, &amount, &e.current_contract_address());
        #[cfg(not(feature = "certora"))]
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
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn burn(e: &Env, user_address: &Address, amount: i128) {
        if amount > Base::balance(e, user_address) {
            panic_with_error!(e, RWAError::InsufficientBalance);
        }

        // Check if we need to unfreeze tokens to complete the burn
        let free_tokens = Self::get_free_tokens(e, user_address);
        if free_tokens < amount {
            let tokens_to_unfreeze = amount - free_tokens;
            let current_frozen = Self::get_frozen_tokens(e, user_address);
            let new_frozen = current_frozen - tokens_to_unfreeze;

            e.storage()
                .persistent()
                .set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
            #[cfg(not(feature = "certora"))]
            emit_tokens_unfrozen(e, user_address, tokens_to_unfreeze);
        }

        Base::update(e, Some(user_address), None, amount);

        let compliance_addr = Self::compliance(e);
        let compliance_client = ComplianceClient::new(e, &compliance_addr);
        compliance_client.destroyed(user_address, &amount, &e.current_contract_address());
        #[cfg(not(feature = "certora"))]
        emit_burn(e, user_address, amount);
    }

    /// Recovery function used to force transfer tokens from a old account to a
    /// new account. This function transfers all tokens and preserves the frozen
    /// status from the old account to the new account. Returns `true` if
    /// recovery was successful, `false` if no tokens to recover.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `old_account` - The address of the wallet that lost access.
    /// * `new_account` - The address of the new account to receive the tokens.
    ///
    /// # Errors
    ///
    /// * [`RWAError::IdentityVerificationFailed`] - When the identity of the
    ///   new account cannot be verified.
    /// * [`RWAError::IdentityMismatch`] - When the new account is not the
    ///   target of the recovery process for the old wallet.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", old_account: Address, new_account: Address]`
    /// * data - `[amount: i128]`
    /// * topics - `["recovery_success", old_account: Address, new_account:
    ///   Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// This function preserves the frozen status (both partial and full) from
    /// the old account and applies it to the new account, maintaining
    /// regulatory compliance.
    ///
    /// This functions does not concern itself with the Identity Management.
    /// If the old account's identity should be removed, it should be done on
    /// the Identity Stack.
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization and compliance
    /// checks. Should only be used by authorized recovery or admin
    /// functions.
    pub fn recover_balance(e: &Env, old_account: &Address, new_account: &Address) -> bool {
        // Verify identity for the new account
        let identity_verifier_addr = Self::identity_verifier(e);
        let identity_verifier_client = IdentityVerifierClient::new(e, &identity_verifier_addr);
        identity_verifier_client.verify_identity(new_account);

        // Verify that the new account is the recovery target for the old account
        let recovery_target = identity_verifier_client
            .recovery_target(old_account)
            .unwrap_or_else(|| panic_with_error!(e, RWAError::IdentityMismatch));

        if recovery_target != *new_account {
            panic_with_error!(e, RWAError::IdentityMismatch);
        }

        // Get the balance of the old account, if there is nothing to transfer, return
        // false
        let lost_balance = Base::balance(e, old_account);
        if lost_balance == 0 {
            return false;
        }

        // Store frozen status before transfer
        let frozen_tokens = Self::get_frozen_tokens(e, old_account);
        let is_address_frozen = Self::is_frozen(e, old_account);

        // Use forced_transfer to transfer all tokens (this handles unfreezing as
        // needed)
        Self::forced_transfer(e, old_account, new_account, lost_balance);

        // Preserve frozen tokens on the new account if there were any
        if frozen_tokens > 0 {
            Self::freeze_partial_tokens(e, new_account, frozen_tokens);
        }

        // Preserve address frozen status on the new account if it was frozen
        if is_address_frozen {
            Self::set_address_frozen(e, new_account, true);
        }
        #[cfg(not(feature = "certora"))]
        emit_recovery_success(e, old_account, new_account);

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
    /// * topics - `["address_frozen", user_address: Address, is_frozen: bool]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_address_frozen(e: &Env, user_address: &Address, freeze: bool) {
        e.storage().persistent().set(&RWAStorageKey::AddressFrozen(user_address.clone()), &freeze);
        #[cfg(not(feature = "certora"))]
        emit_address_frozen(e, user_address, freeze);
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
        #[cfg(not(feature = "certora"))]
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
        #[cfg(not(feature = "certora"))]
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
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_onchain_id(e: &Env, onchain_id: &Address) {
        e.storage().instance().set(&RWAStorageKey::OnchainId, onchain_id);
        #[cfg(not(feature = "certora"))]
        emit_token_onchain_id_updated(e, onchain_id);
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
    /// * topics - `["compliance_set", compliance: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_compliance(e: &Env, compliance: &Address) {
        e.storage().instance().set(&RWAStorageKey::Compliance, compliance);
        #[cfg(not(feature = "certora"))]
        emit_compliance_set(e, compliance);
    }

    /// Sets the identity verifier contract of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_verifier` - The address of the identity verifier contract.
    ///
    /// # Events
    ///
    /// * topics - ["identity_verifier_set", identity_verifier: Address]
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: This function bypasses authorization checks and should
    /// only be used internally or in admin functions that implement their own
    /// authorization logic.
    pub fn set_identity_verifier(e: &Env, identity_verifier: &Address) {
        e.storage().instance().set(&RWAStorageKey::IdentityVerifier, identity_verifier);
        #[cfg(not(feature = "certora"))]
        emit_identity_verifier_set(e, identity_verifier);
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

        let identity_verifier_addr = Self::identity_verifier(e);
        let identity_verifier_client = IdentityVerifierClient::new(e, &identity_verifier_addr);
        identity_verifier_client.verify_identity(from);
        identity_verifier_client.verify_identity(to);

        // Validate compliance rules for the transfer
        let compliance_addr = Self::compliance(e);
        let compliance_client = ComplianceClient::new(e, &compliance_addr);
        let can_transfer: bool =
            compliance_client.can_transfer(from, to, &amount, &e.current_contract_address());

        if !can_transfer {
            panic_with_error!(e, RWAError::TransferNotCompliant);
        }

        Base::update(e, Some(from), Some(to), amount);

        compliance_client.transferred(from, to, &amount, &e.current_contract_address());
        #[cfg(not(feature = "certora"))]
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
