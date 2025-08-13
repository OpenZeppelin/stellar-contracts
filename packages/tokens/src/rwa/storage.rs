use soroban_sdk::{
    contracttype, panic_with_error, symbol_short, Address, Env, IntoVal, Symbol, Vec,
};
use stellar_contract_utils::pausable::{paused, PausableError};

use super::{
    emit_address_frozen, emit_approve, emit_burn, emit_compliance_added,
    emit_identity_verifier_added, emit_mint, emit_recovery_success, emit_token_information_updated,
    emit_tokens_frozen, emit_tokens_unfrozen, emit_transfer,
};
use crate::rwa::{
    RWAError, ALLOWANCE_EXTEND_AMOUNT, ALLOWANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT,
    BALANCE_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT, FROZEN_TTL_THRESHOLD,
};

/// Storage key that maps to [`RWAMetadata`]
pub const METADATA_KEY: Symbol = symbol_short!("METADATA");

/// Storage key that maps to [`AllowanceData`]
#[contracttype]
pub struct AllowanceKey {
    pub owner: Address,
    pub spender: Address,
}

/// Storage container for the amount of tokens for which an allowance is granted
/// and the ledger number at which this allowance expires.
#[contracttype]
pub struct AllowanceData {
    pub amount: i128,
    pub live_until_ledger: u32,
}

/// Storage keys for the data associated with `RWA` token
#[contracttype]
pub enum RWAStorageKey {
    /// Total supply of tokens
    TotalSupply,
    /// Balance of a specific address
    Balance(Address),
    /// Allowance between owner and spender
    Allowance(AllowanceKey),
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
}

/// Storage container for RWA token metadata
#[contracttype]
pub struct RWAMetadata {
    pub decimals: u32,
    pub name: Symbol,
    pub symbol: Symbol,
    pub version: Symbol,
}

// ################## QUERY STATE ##################

/// Returns the total amount of tokens in circulation. If no supply is
/// recorded, it defaults to `0`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn total_supply(e: &Env) -> i128 {
    e.storage().instance().get(&RWAStorageKey::TotalSupply).unwrap_or(0)
}

/// Returns the amount of tokens held by `account`. Defaults to `0` if no
/// balance is stored.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address for which the balance is being queried.
pub fn balance_of(e: &Env, account: &Address) -> i128 {
    let key = RWAStorageKey::Balance(account.clone());
    if let Some(balance) = e.storage().persistent().get::<_, i128>(&key) {
        e.storage().persistent().extend_ttl(&key, BALANCE_TTL_THRESHOLD, BALANCE_EXTEND_AMOUNT);
        balance
    } else {
        0
    }
}

/// Returns the amount of tokens a `spender` is allowed to spend on behalf
/// of an `owner` and the ledger number at which this allowance expires.
/// Both values default to `0`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
///
/// # Notes
///
/// Attention is required when `live_until_ledger` is less than the current
/// ledger number, as this indicates the entry has expired. In such cases,
/// the allowance should be treated as `0`.
pub fn allowance_data(e: &Env, owner: &Address, spender: &Address) -> AllowanceData {
    let key =
        RWAStorageKey::Allowance(AllowanceKey { owner: owner.clone(), spender: spender.clone() });
    if let Some(allowance) = e.storage().temporary().get::<_, AllowanceData>(&key) {
        e.storage().temporary().extend_ttl(&key, ALLOWANCE_TTL_THRESHOLD, ALLOWANCE_EXTEND_AMOUNT);
        allowance
    } else {
        AllowanceData { amount: 0, live_until_ledger: 0 }
    }
}

/// Returns the amount of tokens a `spender` is allowed to spend on behalf
/// of an `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
///
/// # Notes
///
/// An allowance entry where `live_until_ledger` is less than the current
/// ledger number is treated as an allowance with amount `0`.
pub fn allowance(e: &Env, owner: &Address, spender: &Address) -> i128 {
    let allowance = allowance_data(e, owner, spender);

    if allowance.live_until_ledger < e.ledger().sequence() {
        return 0;
    }

    allowance.amount
}

/// Returns the RWA token metadata such as decimals, name, symbol, and version.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`RWAError::UnsetMetadata`] - When trying to access uninitialized
///   metadata.
pub fn get_metadata(e: &Env) -> RWAMetadata {
    e.storage()
        .instance()
        .get(&METADATA_KEY)
        .unwrap_or_else(|| panic_with_error!(e, RWAError::UnsetMetadata))
}

/// Returns the token name.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * refer to [`get_metadata`] errors.
pub fn name(e: &Env) -> Symbol {
    get_metadata(e).name
}

/// Returns the token symbol.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * refer to [`get_metadata`] errors.
pub fn symbol(e: &Env) -> Symbol {
    get_metadata(e).symbol
}

/// Returns the token decimals.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * refer to [`get_metadata`] errors.
pub fn decimals(e: &Env) -> u32 {
    get_metadata(e).decimals
}

/// Returns the token version.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * refer to [`get_metadata`] errors.
pub fn version(e: &Env) -> Symbol {
    get_metadata(e).version
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
/// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is not
///   set.
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
/// * [`RWAError::ComplianceNotSet`] - When the compliance contract is not set.
pub fn compliance(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&RWAStorageKey::Compliance)
        .unwrap_or_else(|| panic_with_error!(e, RWAError::ComplianceNotSet))
}

/// Returns the freezing status of a wallet.
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
/// The amount of frozen tokens is always <= to the total balance of the wallet.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `user_address` - The address of the wallet on which get_frozen_tokens is
///   called.
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
    let total_balance = balance_of(e, user_address);
    let frozen_tokens = get_frozen_tokens(e, user_address);

    // frozen tokens cannot be greater than total balance, necessary checks are done
    // in state changing functions
    total_balance - frozen_tokens
}

// ################## CHANGE STATE ##################

/// Sets the amount of tokens a `spender` is allowed to spend on behalf of
/// an `owner`. Overrides any existing allowance set between `spender`
/// and `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///
/// # Errors
///
/// * refer to [`set_allowance`] errors.
///
/// # Events
///
/// * topics - `["approve", from: Address, spender: Address]`
/// * data - `[amount: i128, live_until_ledger: u32]`
///
/// # Notes
///
/// * Authorization for `owner` is required.
/// * Allowance is implicitly timebound by the maximum allowed storage TTL value
///   which is a network parameter, i.e. one cannot set an allowance for a
///   longer period. This behavior closely mirrors the functioning of the
///   "Stellar Asset Contract" implementation for consistency reasons.
pub fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
    owner.require_auth();
    set_allowance(e, owner, spender, amount, live_until_ledger);
    emit_approve(e, owner, spender, amount, live_until_ledger);
}

/// Sets the amount of tokens a `spender` is allowed to spend on behalf of
/// an `owner`. Overrides any existing allowance set between `spender`
/// and `owner`. Doesn't handle authorization, nor event emission.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens made available to `spender`.
/// * `live_until_ledger` - The ledger number at which the allowance expires.
///   `live_until_ledger`` argument is implicitly bounded by the maximum allowed
///   TTL extension for a temporary storage entry and specifying a higher value
///   will cause the code to panic.
///
/// # Errors
///
/// * [`RWAError::InvalidLiveUntilLedger`] - Occurs when attempting to set
///   `live_until_ledger` that is 1) greater than the maximum allowed or 2) less
///   than the current ledger number and `amount` is greater than `0`.
/// * [`RWAError::LessThanZero`] - Occurs when `amount < 0`.
///
/// # Notes
///
/// * This function does not enforce authorization. Ensure that authorization is
///   handled at a higher level.
/// * Allowance is implicitly timebound by the maximum allowed storage TTL value
///   which is a network parameter, i.e. one cannot set an allowance for a
///   longer period. This behavior closely mirrors the functioning of the
///   "Stellar Asset Contract" implementation for consistency reasons.
pub fn set_allowance(
    e: &Env,
    owner: &Address,
    spender: &Address,
    amount: i128,
    live_until_ledger: u32,
) {
    if amount < 0 {
        panic_with_error!(e, RWAError::LessThanZero);
    }

    let current_ledger = e.ledger().sequence();

    if live_until_ledger > e.ledger().max_live_until_ledger()
        || (amount > 0 && live_until_ledger < current_ledger)
    {
        panic_with_error!(e, RWAError::InvalidLiveUntilLedger);
    }

    let key =
        RWAStorageKey::Allowance(AllowanceKey { owner: owner.clone(), spender: spender.clone() });
    let allowance = AllowanceData { amount, live_until_ledger };

    e.storage().temporary().set(&key, &allowance);

    if amount > 0 {
        // NOTE: cannot revert because of the check above;
        // NOTE: 1 is not added to `live_for` as in the SAC implementation which
        // is a bug tracked in https://github.com/stellar/rs-soroban-env/issues/1519
        let live_for = live_until_ledger - current_ledger;

        e.storage().temporary().extend_ttl(&key, live_for, live_for);
    }
}

/// Deducts the amount of tokens a `spender` is allowed to spend on behalf
/// of an `owner`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `owner` - The address holding the tokens.
/// * `spender` - The address authorized to spend the tokens.
/// * `amount` - The amount of tokens to be deducted from `spender`s allowance.
///
/// # Errors
///
/// * [`RWAError::InsufficientAllowance`] - When attempting to transfer more
///   tokens than `spender` current allowance.
/// * [`RWAError::LessThanZero`] - Occurs when `amount < 0`.
/// * also refer to [`set_allowance`] errors.
///
/// # Notes
///
/// This function does not enforce authorization. Ensure that authorization
/// is handled at a higher level.
pub fn spend_allowance(e: &Env, owner: &Address, spender: &Address, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, RWAError::LessThanZero)
    }

    let allowance_data = allowance_data(e, owner, spender);

    if allowance_data.amount < amount {
        panic_with_error!(e, RWAError::InsufficientAllowance);
    }

    if amount > 0 {
        set_allowance(
            e,
            owner,
            spender,
            allowance_data.amount - amount,
            allowance_data.live_until_ledger,
        );
    }
}

/// Transfers `amount` of tokens from `from` to `to` or alternatively
/// mints (or burns) tokens if `from` (or `to`) is `None`. Updates the total
/// supply accordingly. Includes RWA-specific checks for paused state,
/// frozen addresses, and frozen tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the transferred tokens.
/// * `amount` - The amount of tokens to be transferred.
///
/// # Errors
///
/// * [`RWAError::InsufficientBalance`] - When attempting to transfer more
///   tokens than `from` current balance.
/// * [`RWAError::InsufficientFreeTokens`] - When attempting to transfer frozen
///   tokens.
/// * [`RWAError::LessThanZero`] - When `amount < 0`.
/// * [`RWAError::MathOverflow`] - When `total_supply` overflows.
/// * [`RWAError::AddressFrozen`] - When from or to address is frozen.
/// * [`PausableError::EnforcedPause`] - When the contract is paused.
///
/// # Notes
///
/// This function does not enforce authorization or compliance checks.
/// Ensure that authorization and compliance validation are handled at a higher
/// level.
pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, RWAError::LessThanZero);
    }

    // Check if contract is paused
    if paused(e) {
        panic_with_error!(e, PausableError::EnforcedPause);
    }

    // Check if addresses are frozen
    if let Some(from_addr) = from {
        if is_frozen(e, from_addr) {
            panic_with_error!(e, RWAError::AddressFrozen);
        }
    }
    if let Some(to_addr) = to {
        if is_frozen(e, to_addr) {
            panic_with_error!(e, RWAError::AddressFrozen);
        }
    }

    if let Some(account) = from {
        let mut from_balance = balance_of(e, account);
        if from_balance < amount {
            panic_with_error!(e, RWAError::InsufficientBalance);
        }

        // Check if there are enough free tokens (not frozen)
        let free_tokens = get_free_tokens(e, account);
        if free_tokens < amount {
            panic_with_error!(e, RWAError::InsufficientFreeTokens);
        }

        // NOTE: can't underflow because of the check above.
        from_balance -= amount;
        e.storage().persistent().set(&RWAStorageKey::Balance(account.clone()), &from_balance);
    } else {
        // `from` is None, so we're minting tokens.
        let total_supply = total_supply(e);
        let Some(new_total_supply) = total_supply.checked_add(amount) else {
            panic_with_error!(e, RWAError::MathOverflow);
        };
        e.storage().instance().set(&RWAStorageKey::TotalSupply, &new_total_supply);
    }

    if let Some(account) = to {
        // NOTE: can't overflow because balance + amount is at most total_supply.
        let to_balance = balance_of(e, account) + amount;
        e.storage().persistent().set(&RWAStorageKey::Balance(account.clone()), &to_balance);
    } else {
        // `to` is None, so we're burning tokens.

        // NOTE: can't overflow because amount <= total_supply or amount <= from_balance
        // <= total_supply.
        let total_supply = total_supply(e) - amount;
        e.storage().instance().set(&RWAStorageKey::TotalSupply, &total_supply);
    }
}

/// Transfers `amount` tokens from `from` to `to`.
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
/// * refer to [`update`] errors.
/// * refer to [`validate_compliance`] errors.
/// * refer to [`verify_identity`] errors.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `from` is required.
pub fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    from.require_auth();

    // Verify identity verifier for both addresses
    verify_identity(e, from);
    verify_identity(e, to);

    // Validate compliance rules for the transfer
    validate_compliance(e, from, to, amount);

    update(e, Some(from), Some(to), amount);
    emit_transfer(e, from, to, amount);
}

/// Transfers `amount` tokens from `from` to `to` on behalf of `spender`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The address authorized to spend the tokens.
/// * `from` - The address holding the tokens.
/// * `to` - The address receiving the tokens.
/// * `amount` - The amount of tokens to transfer.
///
/// # Errors
///
/// * refer to [`spend_allowance`] and [`update`] errors.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// Authorization for `spender` is required.
pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
    spender.require_auth();
    spend_allowance(e, from, spender, amount);
    transfer(e, from, to, amount);
}

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
/// * [`RWAError::LessThanZero`] - When `amount < 0`.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// This function bypasses freezing restrictions and can unfreeze tokens
/// as needed. It's intended for regulatory compliance and recovery scenarios.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization and freezing checks.
/// Should only be used by authorized compliance or admin functions.
pub fn forced_transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
    // TODO: check this and compare it with `update`
    if amount < 0 {
        panic_with_error!(e, RWAError::LessThanZero);
    }

    let mut from_balance = balance_of(e, from);
    if from_balance < amount {
        panic_with_error!(e, RWAError::InsufficientBalance);
    }

    // Check if we need to unfreeze tokens to complete the transfer
    let free_tokens = get_free_tokens(e, from);
    if free_tokens < amount {
        let tokens_to_unfreeze = amount - free_tokens;
        let current_frozen = get_frozen_tokens(e, from);
        let new_frozen = current_frozen - tokens_to_unfreeze;

        e.storage().persistent().set(&RWAStorageKey::FrozenTokens(from.clone()), &new_frozen);
        emit_tokens_unfrozen(e, from, tokens_to_unfreeze);
    }

    // Update balances directly (bypassing paused/frozen checks)
    from_balance -= amount;
    let to_balance = balance_of(e, to) + amount;

    e.storage().persistent().set(&RWAStorageKey::Balance(from.clone()), &from_balance);
    e.storage().persistent().set(&RWAStorageKey::Balance(to.clone()), &to_balance);

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
/// refer to [`update`] errors.
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
    verify_identity(e, to);

    update(e, None, Some(to), amount);
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
/// * refer to [`update`] errors.
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
    let free_tokens = get_free_tokens(e, user_address);
    if free_tokens < amount {
        let tokens_to_unfreeze = amount - free_tokens;
        let current_frozen = get_frozen_tokens(e, user_address);
        let new_frozen = current_frozen - tokens_to_unfreeze;

        e.storage()
            .persistent()
            .set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
        emit_tokens_unfrozen(e, user_address, tokens_to_unfreeze);
    }

    update(e, Some(user_address), None, amount);
    emit_burn(e, user_address, amount);
}

/// Recovery function used to force transfer tokens from a lost wallet to a new
/// wallet. This function transfers all tokens and clears frozen status from the
/// lost wallet. Returns `true` if recovery was successful, `false` if no tokens
/// to recover.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `lost_wallet` - The address of the wallet that lost access.
/// * `new_wallet` - The address of the new wallet to receive the tokens.
/// * `investor_onchain_id` - The onchain ID of the investor for verification.
///
/// # Errors
///
/// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is not
///   configured.
/// * [`RWAError::AddressNotVerified`] - When the new wallet is not verified.
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
/// **IMPORTANT**: This function bypasses authorization and compliance checks.
/// Should only be used by authorized recovery or admin functions.
pub fn recovery_address(
    e: &Env,
    lost_wallet: &Address,
    new_wallet: &Address,
    investor_onchain_id: &Address,
) -> bool {
    // Verify identity for the new wallet and investor onchain ID
    verify_identity(e, new_wallet);
    verify_recovery_identity(e, lost_wallet, new_wallet, investor_onchain_id);

    let lost_balance = balance_of(e, lost_wallet);
    if lost_balance == 0 {
        return false;
    }

    // Transfer all tokens from lost wallet to new wallet
    let frozen_tokens = get_frozen_tokens(e, lost_wallet);

    // If there are frozen tokens, unfreeze them first
    if frozen_tokens > 0 {
        e.storage().persistent().set(&RWAStorageKey::FrozenTokens(lost_wallet.clone()), &0i128);
        emit_tokens_unfrozen(e, lost_wallet, frozen_tokens);
    }

    // Transfer all balance
    let new_balance = balance_of(e, new_wallet) + lost_balance;
    e.storage().persistent().set(&RWAStorageKey::Balance(lost_wallet.clone()), &0i128);
    e.storage().persistent().set(&RWAStorageKey::Balance(new_wallet.clone()), &new_balance);

    // Clear frozen status if set
    if is_frozen(e, lost_wallet) {
        e.storage().persistent().set(&RWAStorageKey::AddressFrozen(lost_wallet.clone()), &false);
    }

    emit_transfer(e, lost_wallet, new_wallet, lost_balance);
    emit_recovery_success(e, lost_wallet, new_wallet, investor_onchain_id);

    true
}

/// Sets the frozen status for an address.
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
/// * [`RWAError::InsufficientBalance`] - When trying to freeze more tokens than
///   available.
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

    let current_balance = balance_of(e, user_address);
    let current_frozen = get_frozen_tokens(e, user_address);
    let new_frozen = current_frozen + amount;

    if new_frozen > current_balance {
        panic_with_error!(e, RWAError::InsufficientBalance);
    }

    e.storage().persistent().set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
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
/// * [`RWAError::InsufficientFreeTokens`] - When trying to unfreeze more tokens
///   than are frozen.
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

    let current_frozen = get_frozen_tokens(e, user_address);
    if current_frozen < amount {
        panic_with_error!(e, RWAError::InsufficientFreeTokens);
    }

    let new_frozen = current_frozen - amount;
    e.storage().persistent().set(&RWAStorageKey::FrozenTokens(user_address.clone()), &new_frozen);
    emit_tokens_unfrozen(e, user_address, amount);
}

/// Sets the token name.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `name` - The new name for the token.
///
/// # Events
///
/// * topics - `["token_info", name: Symbol, symbol: Symbol, decimals: u32,
///   version: Symbol, onchain_id: Address]`
/// * data - `[]`
///
/// # Errors
///
/// * [`RWAError::EmptyValue`] - When the name is empty.
/// * refer to [`get_metadata`] errors.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used internally or in admin functions that implement their own
/// authorization logic.
pub fn set_name(e: &Env, name: Symbol) {
    if name == symbol_short!("") {
        panic_with_error!(e, RWAError::EmptyValue);
    }

    let mut metadata = get_metadata(e);
    metadata.name = name;
    e.storage().instance().set(&METADATA_KEY, &metadata);

    emit_token_information_updated(
        e,
        &metadata.name,
        &metadata.symbol,
        metadata.decimals,
        &metadata.version,
        &onchain_id(e),
    );
}

/// Sets the token symbol.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `symbol` - The new symbol for the token.
///
/// # Events
///
/// * topics - `["token_info", name: Symbol, symbol: Symbol, decimals: u32,
///   version: Symbol, onchain_id: Address]`
/// * data - `[]`
///
/// # Errors
///
/// * [`RWAError::EmptyValue`] - When the symbol is empty.
/// * refer to [`get_metadata`] errors.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used internally or in admin functions that implement their own
/// authorization logic.
pub fn set_symbol(e: &Env, symbol: Symbol) {
    if symbol == symbol_short!("") {
        panic_with_error!(e, RWAError::EmptyValue);
    }

    let mut metadata = get_metadata(e);
    metadata.symbol = symbol;
    e.storage().instance().set(&METADATA_KEY, &metadata);

    emit_token_information_updated(
        e,
        &metadata.name,
        &metadata.symbol,
        metadata.decimals,
        &metadata.version,
        &onchain_id(e),
    );
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
/// * refer to [`get_metadata`] errors.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used internally or in admin functions that implement their own
/// authorization logic.
pub fn set_onchain_id(e: &Env, onchain_id: &Address) {
    e.storage().instance().set(&RWAStorageKey::OnchainId, onchain_id);

    let metadata = get_metadata(e);
    emit_token_information_updated(
        e,
        &metadata.name,
        &metadata.symbol,
        metadata.decimals,
        &metadata.version,
        onchain_id,
    );
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

/// Sets the RWA token metadata including decimals, name, symbol, and version.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `decimals` - The number of decimal places for the token.
/// * `name` - The name of the token.
/// * `symbol` - The symbol of the token.
/// * `version` - The version of the token.
///
/// # Errors
///
/// * [`RWAError::EmptyValue`] - When the name, symbol, or version is empty.
///
/// # Notes
///
/// This function is typically used during contract initialization to set
/// the initial metadata. It does not emit events.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used during contract initialization or in admin functions that
/// implement their own authorization logic.
pub fn set_metadata(e: &Env, decimals: u32, name: Symbol, symbol: Symbol, version: Symbol) {
    let empty = symbol_short!("");
    if name == empty || symbol == empty || version == empty {
        panic_with_error!(e, RWAError::EmptyValue);
    }

    let metadata = RWAMetadata { decimals, name, symbol, version };
    e.storage().instance().set(&METADATA_KEY, &metadata);
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
/// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is not
///   configured.
/// * [`RWAError::AddressNotVerified`] - When the address is not verified in the
///   identity verifier.
///
/// # Notes
///
/// This function calls the identity verifier contract to check if the address
/// has a valid, verified identity. The identity verifier should implement a
/// `is_verify` function that returns a boolean.
fn verify_identity(e: &Env, user_address: &Address) {
    let identity_verifier_addr =
        match e.storage().instance().get::<_, Address>(&RWAStorageKey::IdentityVerifier) {
            Some(addr) => addr,
            None => panic_with_error!(e, RWAError::IdentityVerifierNotSet),
        };

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
/// * [`RWAError::TransferNotCompliant`] - When the transfer violates compliance
///   rules.
///
/// # Notes
///
/// This function calls the compliance contract to validate the transfer against
/// configured compliance rules. The compliance contract should implement a
/// `can_xfer` function that returns a boolean.
fn validate_compliance(e: &Env, from: &Address, to: &Address, amount: i128) {
    let compliance_addr = match e.storage().instance().get::<_, Address>(&RWAStorageKey::Compliance)
    {
        Some(addr) => addr,
        None => panic_with_error!(e, RWAError::ComplianceNotSet),
    };

    // Call the compliance contract to validate the transfer
    let can_transfer: bool = e.invoke_contract(
        &compliance_addr,
        &Symbol::new(e, "can_transfer"),
        Vec::from_array(e, [from.into_val(e), to.into_val(e), amount.into_val(e)]),
    );

    if !can_transfer {
        panic_with_error!(e, RWAError::TransferNotCompliant);
    }
}

/// Verifies recovery identity for wallet recovery operations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `lost_wallet` - The address of the lost wallet.
/// * `new_wallet` - The address of the new wallet.
/// * `investor_onchain_id` - The onchain ID of the investor.
///
/// # Errors
///
/// * [`RWAError::IdentityVerifierNotSet`] - When the identity verifier is not
///   configured.
/// * [`RWAError::RecoveryFailed`] - When recovery parameters are invalid.
///
/// # Notes
///
/// This function calls the identity verifier contract to verify that the
/// recovery operation is valid for the given investor onchain ID. The identity
/// verifier should implement a `can_recov` function.
fn verify_recovery_identity(
    e: &Env,
    lost_wallet: &Address,
    new_wallet: &Address,
    investor_onchain_id: &Address,
) {
    let identity_verifier_addr =
        match e.storage().instance().get::<_, Address>(&RWAStorageKey::IdentityVerifier) {
            Some(addr) => addr,
            None => panic_with_error!(e, RWAError::IdentityVerifierNotSet),
        };

    // Call the identity verifier contract to verify recovery eligibility
    let can_recover: bool = e.invoke_contract(
        &identity_verifier_addr,
        &Symbol::new(e, "can_recover"),
        Vec::from_array(
            e,
            [lost_wallet.into_val(e), new_wallet.into_val(e), investor_onchain_id.into_val(e)],
        ),
    );

    if !can_recover {
        panic_with_error!(e, RWAError::RecoveryFailed);
    }
}
