use soroban_sdk::{contracttype, panic_with_error, token::StellarAssetClient, Address, Env};

use crate::FungibleTokenError;

/// Storage key for accessing the SAC address
#[contracttype]
pub enum SACAdminWrapperDataKey {
    Sac,
}

// ################## QUERY STATE ##################

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
        .get(&SACAdminWrapperDataKey::Sac)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACNotSet))
}

/// Returns a SAC client, initialized with the stored SAC address.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
///
/// # Errors
///
/// * refer to [`get_sac_address`] errors.
pub fn get_sac_client<'a>(e: &Env) -> StellarAssetClient<'a> {
    let sac_address = get_sac_address(e);
    StellarAssetClient::new(e, &sac_address)
}

// ################## CHANGE STATE ##################

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
    e.storage().instance().set(&SACAdminWrapperDataKey::Sac, &sac);
}

/// Sets the administrator to the specified address `new_admin`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `new_admin` - The address which will henceforth be the administrator of
///   the token contract.
///
/// # Errors
///
/// * refer to [`get_sac_client`] errors.
///
/// # Security Warning
///
/// This function lacks authorization checks. The implementer MUST assure proper
/// access control and authorization.
pub fn set_admin(e: &Env, new_admin: &Address) {
    let client = get_sac_client(e);
    client.set_admin(new_admin);
}

/// Mints `amount` to `to`.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address which will receive the minted tokens.
/// * `amount` - The amount of tokens to be minted.
///
/// # Errors
///
/// * refer to [`get_sac_client`] errors.
///
/// # Security Warning
///
/// This function lacks authorization checks. The implementer MUST assure proper
/// access control and authorization.
pub fn mint(e: &Env, to: &Address, amount: i128) {
    let client = get_sac_client(e);
    client.mint(to, &amount);
}

/// Sets whether the account is authorized to use its balance. If
/// `authorized` is true, `id` should be able to use its balance.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `id` - The address being (de-)authorized.
/// * `authorize` - Whether or not `id` can use its balance.
///
/// # Errors
///
/// * refer to [`get_sac_client`] errors.
///
/// # Security Warning
///
/// This function lacks authorization checks. The implementer MUST assure proper
/// access control and authorization.
pub fn set_authorized(e: &Env, id: &Address, authorize: bool) {
    let client = get_sac_client(e);
    client.set_authorized(id, &authorize);
}

/// Clawback `amount` from `from` account. `amount` is burned in the
/// clawback process.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `from` - The address holding the balance from which the clawback will take
///   tokens.
/// * `amount` - The amount of tokens to be clawed back.
///
/// # Errors
///
/// * refer to [`get_sac_client`] errors.
///
/// # Security Warning
///
/// This function lacks authorization checks. The implementer MUST assure proper
/// access control and authorization.
pub fn clawback(e: &Env, from: &Address, amount: i128) {
    let client = get_sac_client(e);
    client.clawback(from, &amount);
}
