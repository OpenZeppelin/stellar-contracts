use soroban_sdk::{
    auth::{Context, ContractContext},
    contracttype, panic_with_error, Address, Env, Symbol, TryFromVal, Val,
};

use crate::FungibleTokenError;

/// Storage key for accessing the SAC address
#[contracttype]
pub enum SACAdminGenericDataKey {
    Sac,
}

pub const MINT_AMOUNT_INDEX: u32 = 2;
pub const CLAWBACK_AMOUNT_INDEX: u32 = 2;
pub const SET_AUTHORIZED_BOOL_INDEX: u32 = 1;

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

pub fn extract_context(e: &Env, context: &Context) -> SacFn {
    let contract_context = match context {
        Context::Contract(c) => c,
        _ => panic_with_error!(e, FungibleTokenError::SACInvalidContext),
    };

    let sac_addr: Address = e
        .storage()
        .instance()
        .get(&SACAdminGenericDataKey::Sac)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACNotSet));

    if contract_context.contract != sac_addr {
        panic_with_error!(e, FungibleTokenError::SACInvalidContext);
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
