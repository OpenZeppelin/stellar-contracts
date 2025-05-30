use soroban_sdk::{
    auth::ContractContext, contracttype, panic_with_error, Address, Env, Symbol, TryFromVal, Val,
};

use crate::FungibleTokenError;

/// Storage key for accessing the SAC address
#[contracttype]
pub enum SACAdminGenericDataKey {
    Sac,
}

/// Index of `amount` in `fn mint(e: Env, to: Address, amount: i128)`
pub const MINT_AMOUNT_INDEX: u32 = 2;
/// Index of `amount` in `fn clawback(e: Env, from: Address, amount: i128)`
pub const CLAWBACK_AMOUNT_INDEX: u32 = 2;
/// Index of `authorized` in `fn set_authorized(e: Env, authorized: bool,
/// account: Address)`
pub const SET_AUTHORIZED_BOOL_INDEX: u32 = 1;

/// Container mapping the extracted values from a SAC `ContractContext`
pub enum SacFn {
    Mint(i128),
    Clawback(i128),
    SetAuthorized(bool),
    SetAdmin,
    Unknown,
}

/// Returns the stored SAC address.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::SACNotSet`] - Occurs when the SAC address wasn't set
///   beforehand.
pub fn get_sac_address(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&SACAdminGenericDataKey::Sac)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACNotSet))
}

/// A helper function that extracts some elements from `ContractContext` passed
/// in to `__check_auth` of a SAC Admin Generic contract.
///
/// # Returns
///
/// This function wraps and returns the extracted elements in a minimal form,
/// deemed necessary to perform a validation.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `contract_context` - `ContractContext` passed to `__check_auth`.
///
/// # Errors
///
/// * [`FungibleTokenError::SACNotSet`] - Occurs when the SAC address wasn't set
///   beforehand.
/// * [`FungibleTokenError::SACAddressMismatch`] - Occurs when the SAC address
///   doesn't match with the one from `ContractContext`.
/// * refer to [`get_fn_param`] errors.
pub fn extract_sac_contract_context(e: &Env, contract_context: &ContractContext) -> SacFn {
    let sac_addr: Address = e
        .storage()
        .instance()
        .get(&SACAdminGenericDataKey::Sac)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACNotSet));

    if contract_context.contract != sac_addr {
        panic_with_error!(e, FungibleTokenError::SACAddressMismatch);
    }

    if contract_context.fn_name == Symbol::new(e, "mint") {
        let amount = get_fn_param(e, contract_context, MINT_AMOUNT_INDEX);
        SacFn::Mint(amount)
    } else if contract_context.fn_name == Symbol::new(e, "clawback") {
        let amount = get_fn_param(e, contract_context, CLAWBACK_AMOUNT_INDEX);
        SacFn::Clawback(amount)
    } else if contract_context.fn_name == Symbol::new(e, "set_authorized") {
        let authorized = get_fn_param(e, contract_context, SET_AUTHORIZED_BOOL_INDEX);
        SacFn::SetAuthorized(authorized)
    } else if contract_context.fn_name == Symbol::new(e, "set_admin") {
        SacFn::SetAdmin
    } else {
        SacFn::Unknown
    }
}

/// A helper function that extracts a parameter given its `index` in the
/// parameter list of a function from the SAC admin interface.
///
/// # Returns
///
/// The element that was extracted.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `contract_context` - `ContractContext` passed to `__check_auth`.
/// * `index` - The expected position of the function parameter.
///
/// # Errors
///
/// * [`FungibleTokenError::SACMissingFnParam`] - Occurs when `ContractContext`
///   is missing an expected function parameter for a given admin function of
///   the SAC interface.
/// * [`FungibleTokenError::SACInvalidFnParam`] - Occurs when an function
///   parameter extracted from `ContractContext` can't be transformed to the
///   expected type.
pub fn get_fn_param<V: TryFromVal<Env, Val>>(
    e: &Env,
    contract_context: &ContractContext,
    index: u32,
) -> V {
    let val = contract_context
        .args
        .get(index)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACMissingFnParam));
    V::try_from_val(e, &val)
        .unwrap_or_else(|_| panic_with_error!(e, FungibleTokenError::SACInvalidFnParam))
}

/// Stores the SAC address, typically called from the constructor.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `sac` - The address the SAC contract.
///
/// # Security Warning
///
/// This function lacks authorization checks. The implementer MUST assure proper
/// access control and authorization.
pub fn set_sac_address(e: &Env, sac: &Address) {
    e.storage().instance().set(&SACAdminGenericDataKey::Sac, &sac);
}
