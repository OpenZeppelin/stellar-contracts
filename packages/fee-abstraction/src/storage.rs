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
    /// Allowed fee token, maps to Address
    Token(u32),
    /// Index assigned to an allowed fee token address
    TokenIndex(Address),
}

/// Collect a fee from the user in a given token and invoke a target contract
/// function.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `fee_token` - The token address to pay the fee with.
/// * `fee_amount` - The actual fee amount to charge.
/// * `max_fee_amount` - The maximum fee amount the user approved.
/// * `expiration_ledger` - The ledger sequence at which the approval expires.
/// * `target_contract` - The contract address to invoke after collecting the
///   fee.
/// * `target_fn` - The function to invoke on the target contract.
/// * `target_args` - The arguments to pass to the target contract function.
/// * `user` - The address of the user paying the fee and authorizing the call.
/// * `fee_recipient` - The address that receives the collected fee.
///
/// # Errors
///
/// * [`FeeAbstractionError::InvalidFeeBounds`] - If `fee_amount >
///   max_fee_amount`.
/// * [`FeeAbstractionError::FeeTokenNotAllowed`] - If the fee token is not
///   allowed when the allowlist is enabled.
///
/// # Events
///
/// * topics - `["FeeCollected", user: Address, recipient: Address]`
/// * data - `[token: Address, amount: i128]`
/// * topics - `["ForwardExecuted", user: Address, target_contract: Address]`
/// * data - `[target_fn: Symbol, target_args: Vec<Val>]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function performs authorization checks **only** on the
/// user's input. The contract using this function might need to perform further
/// checks to validate the other parameters. Additionally, the invoker **MUST**
/// ensure the call to the target contract is safe for them.
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

    collect_fee(e, fee_token, fee_amount, max_fee_amount, expiration_ledger, user, fee_recipient);

    let res = e.invoke_contract::<Val>(target_contract, target_fn, target_args.clone());

    emit_forward_executed(e, user, target_contract, target_fn, target_args);

    res
}

/// Collect a fee from the user in a given token.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `fee_token` - The token address to pay the fee with.
/// * `fee_amount` - The actual fee amount to charge.
/// * `max_fee_amount` - The maximum fee amount the user approved.
/// * `expiration_ledger` - The ledger sequence at which the approval expires.
/// * `user` - The address of the user paying the fee.
/// * `recipient` - The address that receives the collected fee.
///
/// # Events
///
/// * topics - `["FeeCollected", user: Address, recipient: Address]`
/// * data - `[token: Address, amount: i128]`
///
/// # Errors
///
/// * [`FeeAbstractionError::InvalidFeeBounds`] - If `fee_amount >
///   max_fee_amount`.
/// * [`FeeAbstractionError::FeeTokenNotAllowed`] - If the fee token is not
///   allowed when the allowlist is enabled.
pub fn collect_fee(
    e: &Env,
    fee_token: &Address,
    fee_amount: i128,
    max_fee_amount: i128,
    expiration_ledger: u32,
    user: &Address,
    recipient: &Address,
) {
    check_allowed_fee_token(e, fee_token);

    // Checking explicitly the fee bounds instead of relying on validations in
    // `approve` + `transfer_from`. There might be another approval from the `user`
    // that will allow charging more than `max_fee_amount` intended for this
    // invocation.
    validate_fee_bounds(e, fee_amount, max_fee_amount);

    let token_client = TokenClient::new(e, fee_token);
    // User signs an approval for `max_fee_amount` so that this contract can charge
    // up to that amount.
    token_client.approve(user, &e.current_contract_address(), &max_fee_amount, &expiration_ledger);

    token_client.transfer_from(&e.current_contract_address(), user, recipient, &fee_amount);

    emit_fee_collected(e, user, recipient, fee_token, fee_amount);
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

        // Can't underflow, it would've been cought be the above panic_with_error
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
///
/// # Errors
///
/// * [`FeeAbstractionError::FeeTokenNotAllowed`] - If the token is not allowed.
pub fn check_allowed_fee_token(e: &Env, token: &Address) {
    if !is_fee_token_allowlist_enabled(e) {
        return;
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
    } else {
        panic_with_error!(e, FeeAbstractionError::FeeTokenNotAllowed);
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
