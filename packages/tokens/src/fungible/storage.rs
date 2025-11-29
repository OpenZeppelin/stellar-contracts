use soroban_sdk::{contracttype, panic_with_error, symbol_short, Address, Env, String, Symbol};

#[cfg(not(feature = "certora"))]
use crate::fungible::{
    emit_approve, emit_mint, emit_transfer,};


use crate::fungible::{
    Base, FungibleTokenError, BALANCE_EXTEND_AMOUNT,
    BALANCE_TTL_THRESHOLD,
};

/// Storage key that maps to [`Metadata`]
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

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    TotalSupply,
    Balance(Address),
    Allowance(AllowanceKey),
}

/// Storage container for token metadata
#[contracttype]
pub struct Metadata {
    pub decimals: u32,
    pub name: String,
    pub symbol: String,
}

impl Base {
    // ################## QUERY STATE ##################

    /// Returns the total amount of tokens in circulation. If no supply is
    /// recorded, it defaults to `0`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn total_supply(e: &Env) -> i128 {
        e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
    }

    /// Returns the amount of tokens held by `account`. Defaults to `0` if no
    /// balance is stored.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address for which the balance is being queried.
    pub fn balance(e: &Env, account: &Address) -> i128 {
        let key = StorageKey::Balance(account.clone());
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
        let key = AllowanceKey { owner: owner.clone(), spender: spender.clone() };
        e.storage()
            .temporary()
            .get(&StorageKey::Allowance(key))
            .unwrap_or(AllowanceData { amount: 0, live_until_ledger: 0 })
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
        let allowance = Base::allowance_data(e, owner, spender);

        if allowance.live_until_ledger < e.ledger().sequence() {
            return 0;
        }

        allowance.amount
    }

    /// Returns the token metadata such as decimals, name and symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UnsetMetadata`] - When trying to access
    ///   uninitialized metadata.
    pub fn get_metadata(e: &Env) -> Metadata {
        e.storage()
            .instance()
            .get(&METADATA_KEY)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::UnsetMetadata))
    }

    /// Returns the token decimals.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::get_metadata`] errors.
    pub fn decimals(e: &Env) -> u32 {
        Base::get_metadata(e).decimals
    }

    /// Returns the token name.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::get_metadata`] errors.
    pub fn name(e: &Env) -> String {
        Base::get_metadata(e).name
    }

    /// Returns the token symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::get_metadata`] errors.
    pub fn symbol(e: &Env) -> String {
        Base::get_metadata(e).symbol
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
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::set_allowance`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["approve", from: Address, spender: Address]`
    /// * data - `[amount: i128, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for `owner` is required.
    /// * Allowance is implicitly timebound by the maximum allowed storage TTL
    ///   value which is a network parameter, i.e. one cannot set an allowance
    ///   for a longer period. This behavior closely mirrors the functioning of
    ///   the "Stellar Asset Contract" implementation for consistency reasons.
    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        owner.require_auth();
        Base::set_allowance(e, owner, spender, amount, live_until_ledger);
        #[cfg(not(feature = "certora"))]
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
    /// * `live_until_ledger` - The ledger number at which the allowance
    ///   expires. `live_until_ledger`` argument is implicitly bounded by the
    ///   maximum allowed TTL extension for a temporary storage entry and
    ///   specifying a higher value will cause the code to panic.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InvalidLiveUntilLedger`] - Occurs when
    ///   attempting to set `live_until_ledger` that is 1) greater than the
    ///   maximum allowed or 2) less than the current ledger number and `amount`
    ///   is greater than `0`.
    /// * [`FungibleTokenError::LessThanZero`] - Occurs when `amount < 0`.
    ///
    /// # Notes
    ///
    /// * This function does not enforce authorization. Ensure that
    ///   authorization is handled at a higher level.
    /// * Allowance is implicitly timebound by the maximum allowed storage TTL
    ///   value which is a network parameter, i.e. one cannot set an allowance
    ///   for a longer period. This behavior closely mirrors the functioning of
    ///   the "Stellar Asset Contract" implementation for consistency reasons.
    pub fn set_allowance(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        if amount < 0 {
            panic_with_error!(e, FungibleTokenError::LessThanZero);
        }

        let current_ledger = e.ledger().sequence();

        if live_until_ledger > e.ledger().max_live_until_ledger()
            || (amount > 0 && live_until_ledger < current_ledger)
        {
            panic_with_error!(e, FungibleTokenError::InvalidLiveUntilLedger);
        }

        let key =
            StorageKey::Allowance(AllowanceKey { owner: owner.clone(), spender: spender.clone() });
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
    /// * `amount` - The amount of tokens to be deducted from `spender`s
    ///   allowance.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::InsufficientAllowance`] - When attempting to
    ///   transfer more tokens than `spender` current allowance.
    /// * [`FungibleTokenError::LessThanZero`] - Occurs when `amount < 0`.
    /// * also refer to [`Base::set_allowance`] errors.
    ///
    /// # Notes
    ///
    /// This function does not enforce authorization. Ensure that authorization
    /// is handled at a higher level.
    pub fn spend_allowance(e: &Env, owner: &Address, spender: &Address, amount: i128) {
        if amount < 0 {
            panic_with_error!(e, FungibleTokenError::LessThanZero)
        }

        let allowance = Base::allowance_data(e, owner, spender);

        if allowance.amount < amount {
            panic_with_error!(e, FungibleTokenError::InsufficientAllowance);
        }

        if amount > 0 {
            Base::set_allowance(
                e,
                owner,
                spender,
                allowance.amount - amount,
                allowance.live_until_ledger,
            );
        }
    }

    /// Transfers `amount` of tokens from `from` to `to`.
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
    /// * refer to [`Base::update`] errors.
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
        Base::update(e, Some(from), Some(to), amount);
        #[cfg(not(feature = "certora"))]
        emit_transfer(e, from, to, amount);
    }

    /// Transfers `amount` of tokens from `from` to `to` using the
    /// allowance mechanism. `amount` is then deducted from `spender`s
    /// allowance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `spender` - The address authorizing the transfer, and having its
    ///   allowance consumed during the transfer.
    /// * `from` - The address holding the tokens which will be transferred.
    /// * `to` - The address receiving the transferred tokens.
    /// * `amount` - The amount of tokens to be transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::spend_allowance`] errors.
    /// * refer to [`Base::update`] errors.
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
        Base::spend_allowance(e, from, spender, amount);
        Base::update(e, Some(from), Some(to), amount);
        #[cfg(not(feature = "certora"))]
        emit_transfer(e, from, to, amount);
    }

    /// Transfers `amount` of tokens from `from` to `to` or alternatively
    /// mints (or burns) tokens if `from` (or `to`) is `None`. Updates the total
    /// supply accordingly.
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
    /// * [`FungibleTokenError::InsufficientBalance`] - When attempting to
    ///   transfer more tokens than `from` current balance.
    /// * [`FungibleTokenError::LessThanZero`] - When `amount < 0`.
    /// * [`FungibleTokenError::MathOverflow`] - When `total_supply` overflows.
    ///
    /// # Notes
    ///
    /// This function does not enforce authorization. Ensure that authorization
    /// is handled at a higher level.
    pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: i128) {
        if amount < 0 {
            panic_with_error!(e, FungibleTokenError::LessThanZero);
        }
        if let Some(account) = from {
            let mut from_balance = Base::balance(e, account);
            if from_balance < amount {
                panic_with_error!(e, FungibleTokenError::InsufficientBalance);
            }
            // NOTE: can't underflow because of the check above.
            from_balance -= amount;
            e.storage().persistent().set(&StorageKey::Balance(account.clone()), &from_balance);
        } else {
            // `from` is None, so we're minting tokens.
            let total_supply = Base::total_supply(e);
            let Some(new_total_supply) = total_supply.checked_add(amount) else {
                panic_with_error!(e, FungibleTokenError::MathOverflow);
            };
            e.storage().instance().set(&StorageKey::TotalSupply, &new_total_supply);
        }

        if let Some(account) = to {
            // NOTE: can't overflow because balance + amount is at most total_supply.
            let to_balance = Base::balance(e, account) + amount;
            e.storage().persistent().set(&StorageKey::Balance(account.clone()), &to_balance);
        } else {
            // `to` is None, so we're burning tokens.

            // NOTE: can't overflow because amount <= total_supply or amount <= from_balance
            // <= total_supply.
            let total_supply = Base::total_supply(e) - amount;
            e.storage().instance().set(&StorageKey::TotalSupply, &total_supply);
        }
    }

    /// Creates `amount` of tokens and assigns them to `to`. Updates
    /// the total supply accordingly.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address receiving the new tokens.
    /// * `amount` - The amount of tokens to mint.
    ///
    /// # Errors
    ///
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
        Base::update(e, None, Some(to), amount);
        #[cfg(not(feature = "certora"))]
        emit_mint(e, to, amount);
    }

    /// Sets the token metadata such as decimals, name and symbol.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `decimals` - The number of decimals.
    /// * `name` - The name of the token.
    /// * `symbol` - The symbol of the token.
    ///
    /// # Notes
    ///
    /// **IMPORTANT**: This function lacks authorization controls. You want to
    /// invoke it most likely from a constructor or from another function with
    /// admin-only authorization.
    pub fn set_metadata(e: &Env, decimals: u32, name: String, symbol: String) {
        let metadata = Metadata { decimals, name, symbol };
        e.storage().instance().set(&METADATA_KEY, &metadata);
    }
}
