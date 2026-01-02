use soroban_sdk::{
    contracttype, panic_with_error, token::TokenClient, Address, Env, IntoVal, Symbol, Val, Vec,
};

use crate::{
    emit_fee_collected, emit_fee_token_allowlist_updated, emit_forward_executed, emit_tokens_swept,
    FeeAbstractionError, FEE_ABSTRACTION_EXTEND_AMOUNT, FEE_ABSTRACTION_TTL_THRESHOLD,
};

// ################## STORAGE KEYS ##################

#[derive(Clone)]
#[contracttype]
pub enum FeeAbstractionStorageKey {
    /// Number of allowed fee tokens
    Count,
    /// Index of allowed fee token mapping to the token address
    Token(u32),
    /// Address of allowed fee token mapping to the assigned index
    TokenIndex(Address),
}

/// Approval strategy for fee collection helpers.
#[derive(Clone, Copy)]
#[contracttype]
pub enum FeeAbstractionApproval {
    /// Only approve `max_fee_amount` if the existing allowance is insufficient.
    Lazy,
    /// Always approve `max_fee_amount`, overwriting previous allowances.
    Eager,
}

// ################## INVOKE TARGET (FORWARD) AND COLLECT FEE ##################

///  Collect the fee and invoke the target contract (forward).
///
/// User's authorization needs to include the token approval as a
/// sub-invocation, alongside the target call if required, as demonstrated in
/// `examples/fee-forwarder-permissionless` and
/// `examples/fee-forwarder-permissioned`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `fee_token` - The token address to pay the fee with.
/// * `fee_amount` - The actual fee amount to charge.
/// * `max_fee_amount` - The maximum fee amount the user approved.
/// * `expiration_ledger` - The ledger sequence at which the approval expires.
/// * `target_contract` - The contract address to invoke.
/// * `target_fn` - The function to invoke on the target contract.
/// * `target_args` - The arguments to pass to the target contract function.
/// * `user` - The address of the user authorizing the call and paying the fee.
/// * `fee_recipient` - The address that receives the collected fee.
/// * `approval` - The approval strategy to use (`Lazy` or `Eager`).
///
/// # Events
///
/// * topics - `["ForwardExecuted", user: Address, target_contract: Address]`
/// * data - `[target_fn: Symbol, target_args: Vec<Val>]`
///
/// * topics - `["FeeCollected", user: Address, recipient: Address]`
/// * data - `[token: Address, amount: i128]`
///
/// # Errors
///
/// * refer to [`collect_fee`] errors.
///
/// # Security Warning
///
/// **IMPORTANT**: This function performs authorization checks **only** on the
/// user's input. The contract using this function should perform further
/// checks and authorization verifications. Additionally, the invoker **MUST**
/// ensure the call to the target contract is safe for them. In the most cases,
/// the latter is to be done off-chain by simulating the outcome of the
/// transaction.
#[allow(clippy::too_many_arguments)]
pub fn collect_fee_and_invoke(
    e: &Env,
    fee_token: &Address,
    fee_amount: i128,
    max_fee_amount: i128,
    expiration_ledger: u32,
    target_contract: &Address,
    target_fn: &Symbol,
    target_args: &Vec<Val>,
    user: &Address,
    fee_recipient: &Address,
    approval: FeeAbstractionApproval,
) -> Val {
    let user_args_for_auth = (
        fee_token.clone(),
        max_fee_amount,
        expiration_ledger,
        target_contract.clone(),
        target_fn.clone(),
        target_args.clone(),
    )
        .into_val(e);
    user.require_auth_for_args(user_args_for_auth);

    collect_fee(
        e,
        fee_token,
        fee_amount,
        max_fee_amount,
        expiration_ledger,
        user,
        fee_recipient,
        approval,
    );

    let res = e.invoke_contract::<Val>(target_contract, target_fn, target_args.clone());

    emit_forward_executed(e, user, target_contract, target_fn, target_args);

    res
}

/// Low-level helper to collect a fee from the user in a given token by checking
/// whether the token is allowed when allow list is enabled.
///
/// It can be used with either eager or lazy approval semantics. `Eager` always
/// approves `max_fee_amount` (overwriting any existing allowance); `Lazy` only
/// approves if the current allowance is less than `max_fee_amount`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `fee_token` - The token address to pay the fee with.
/// * `fee_amount` - The actual fee amount to charge.
/// * `max_fee_amount` - The maximum fee amount the user approved.
/// * `expiration_ledger` - The ledger sequence at which the approval expires.
/// * `user` - The address of the user paying the fee.
/// * `fee_recipient` - The address that receives the collected fee.
/// * `approval` - The approval strategy to use (`Lazy` or `Eager`).
///
/// # Events
///
/// * topics - `["FeeCollected", user: Address, recipient: Address]`
/// * data - `[token: Address, amount: i128]`
///
/// # Errors
///
/// * [`FeeAbstractionError::FeeTokenNotAllowed`] - If the token is not allowed.
/// * [`FeeAbstractionError::InvalidUser`] - If user is current contract.
/// * refer to [`validate_fee_bounds`] errors.
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
/// Additionally, this function does NOT relate to a specific target invocation
/// and should NOT be used in isolation, e.g. making the target contract
/// invocation and the fee collection in different transactions. It is strongly
/// recommended that the fee collection and the target invocation be atomic.
#[allow(clippy::too_many_arguments)]
pub fn collect_fee(
    e: &Env,
    fee_token: &Address,
    fee_amount: i128,
    max_fee_amount: i128,
    expiration_ledger: u32,
    user: &Address,
    fee_recipient: &Address,
    approval: FeeAbstractionApproval,
) {
    if !is_allowed_fee_token(e, fee_token) {
        panic_with_error!(e, FeeAbstractionError::FeeTokenNotAllowed);
    }

    if e.current_contract_address() == *user {
        panic_with_error!(e, FeeAbstractionError::InvalidUser)
    }

    validate_fee_bounds(e, fee_amount, max_fee_amount);

    let token_client = TokenClient::new(e, fee_token);

    match approval {
        FeeAbstractionApproval::Eager => {
            token_client.approve(
                user,
                &e.current_contract_address(),
                &max_fee_amount,
                &expiration_ledger,
            );
        }
        FeeAbstractionApproval::Lazy => {
            let allowance = token_client.allowance(user, &e.current_contract_address());
            if allowance < max_fee_amount {
                token_client.approve(
                    user,
                    &e.current_contract_address(),
                    &max_fee_amount,
                    &expiration_ledger,
                );
            }
        }
    }

    token_client.transfer_from(&e.current_contract_address(), user, fee_recipient, &fee_amount);

    emit_fee_collected(e, user, fee_recipient, fee_token, fee_amount);
}

// ################## FEE TOKEN ALLOWLIST ##################

/// Check if the fee token allowlist is enabled. It is considered enabled if at
/// least one fee token has been added to the allowlist.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Returns
///
/// `true` if the allowlist is enabled, `false` otherwise.
pub fn is_fee_token_allowlist_enabled(e: &Env) -> bool {
    let key = FeeAbstractionStorageKey::Count;
    let count: u32 = e.storage().instance().get(&key).unwrap_or(0);
    count > 0
}

/// Allow or disallow a token for fee payment.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token` - The token contract address.
/// * `allowed` - Whether to allow the token for fee payment.
///
/// # Events
///
/// * topics - `["FeeTokenAllowlistUpdated", token: Address]`
/// * data - `[allowed: bool]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn set_allowed_fee_token(e: &Env, token: &Address, allowed: bool) {
    let count_key = FeeAbstractionStorageKey::Count;
    let mut count: u32 = e.storage().instance().get(&count_key).unwrap_or(0);

    let token_index_key = FeeAbstractionStorageKey::TokenIndex(token.clone());
    let existing_index: Option<u32> = e.storage().persistent().get(&token_index_key);

    if allowed {
        if existing_index.is_some() {
            // Trying to allow an already-allowed token.
            panic_with_error!(e, FeeAbstractionError::FeeTokenAlreadyAllowed);
        }

        // Assign new index at the end.
        e.storage().persistent().set(&FeeAbstractionStorageKey::Token(count), token);
        e.storage().persistent().set(&token_index_key, &count);

        // Increment count.
        count = count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, FeeAbstractionError::TokenCountOverflow));
        e.storage().instance().set(&count_key, &count);
    } else {
        let remove_index = existing_index
            .unwrap_or_else(|| panic_with_error!(e, FeeAbstractionError::FeeTokenNotAllowed));

        // Can't underflow, it would've been caught be the above panic_with_error
        let last_index = count - 1;
        let last_key = FeeAbstractionStorageKey::Token(last_index);

        // Swap and pop
        if remove_index != last_index {
            // Move last token into the removed slot.
            let last_token: Address =
                e.storage().persistent().get(&last_key).expect("last token to be present");

            e.storage()
                .persistent()
                .set(&FeeAbstractionStorageKey::Token(remove_index), &last_token);

            // Update moved token's index mapping.
            e.storage()
                .persistent()
                .set(&FeeAbstractionStorageKey::TokenIndex(last_token.clone()), &remove_index);
        }

        // Remove last index entry.
        e.storage().persistent().remove(&last_key);

        // Remove mapping for the removed token.
        e.storage().persistent().remove(&token_index_key);

        count -= 1;
        e.storage().instance().set(&count_key, &count);
    }

    emit_fee_token_allowlist_updated(e, token, allowed);
}

/// Check if a token is allowed for fee payment.
///
/// If the allowlist is disabled (no fee tokens added), all tokens are
/// considered allowed. If the allowlist is enabled (at least one fee token is
/// added), only explicitly allowed tokens are permitted.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token` - The token contract address to check.
pub fn is_allowed_fee_token(e: &Env, token: &Address) -> bool {
    if !is_fee_token_allowlist_enabled(e) {
        return true;
    }

    let token_index_key = FeeAbstractionStorageKey::TokenIndex(token.clone());
    if let Some(index) = e.storage().persistent().get(&token_index_key) {
        // Extend both persistent entries for token
        e.storage().persistent().extend_ttl(
            &token_index_key,
            FEE_ABSTRACTION_TTL_THRESHOLD,
            FEE_ABSTRACTION_EXTEND_AMOUNT,
        );
        e.storage().persistent().extend_ttl(
            &FeeAbstractionStorageKey::Token(index),
            FEE_ABSTRACTION_TTL_THRESHOLD,
            FEE_ABSTRACTION_EXTEND_AMOUNT,
        );
        true
    } else {
        false
    }
}

// ################## TOKEN SWEEPING ##################

/// Sweep accumulated tokens from the contract to a recipient.
///
/// This is useful when fees are accumulated in this contract with the intention
/// to be transferred occasionally to the intended recipient.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `token` - The token contract address to sweep.
/// * `recipient` - The address to receive the swept tokens.
///
/// # Returns
///
/// The amount of tokens swept.
///
/// # Errors
///
/// * [`FeeAbstractionError::NoTokensToSweep`] - If the contract has no balance
///   of the token.
///
/// # Events
///
/// * topics - `["TokensSwept", token: Address, recipient: Address]`
/// * data - `[amount: i128]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function does not perform authorization checks.
/// The caller must ensure proper authorization before calling this function.
pub fn sweep_token(e: &Env, token: &Address, recipient: &Address) -> i128 {
    let token_client = TokenClient::new(e, token);
    let contract_address = e.current_contract_address();
    let balance = token_client.balance(&contract_address);

    if balance == 0 {
        panic_with_error!(e, FeeAbstractionError::NoTokensToSweep);
    }

    token_client.transfer(&contract_address, recipient, &balance);
    emit_tokens_swept(e, token, recipient, balance);

    balance
}

// ################## VALIDATION HELPERS ##################

/// Validate that the fee amount does not exceed the maximum allowed or is <= 0.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `fee_amount` - The actual fee amount to charge.
/// * `max_fee_amount` - The maximum fee amount the user authorized.
///
/// # Errors
///
/// * [`FeeAbstractionError::InvalidFeeBounds`] - If amounts <= 0 or `fee_amount
///   > max_fee_amount`.
pub fn validate_fee_bounds(e: &Env, fee_amount: i128, max_fee_amount: i128) {
    if fee_amount <= 0 || fee_amount > max_fee_amount {
        panic_with_error!(e, FeeAbstractionError::InvalidFeeBounds);
    }
}
