use soroban_sdk::{contracttype, panic_with_error, token::StellarAssetClient, Address, Env};

use crate::FungibleTokenError;

#[contracttype]
pub enum SacDataKey {
    Sac,
}

pub fn set_sac_address(e: &Env, sac: &Address) {
    e.storage().instance().set(&SacDataKey::Sac, &sac);
}

pub fn get_sac_address(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&SacDataKey::Sac)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::SACNotSet))
}

pub fn get_sac_client<'a>(e: &Env) -> StellarAssetClient<'a> {
    let sac_address = get_sac_address(e);
    StellarAssetClient::new(e, &sac_address)
}

pub fn set_admin(e: &Env, new_admin: &Address) {
    let client = get_sac_client(e);
    client.set_admin(new_admin);
}

pub fn mint(e: &Env, to: &Address, amount: i128) {
    let client = get_sac_client(e);
    client.mint(to, &amount);
}

pub fn set_authorized(e: &Env, id: &Address, authorize: bool) {
    let client = get_sac_client(e);
    client.set_authorized(id, &authorize);
}

pub fn clawback(e: &Env, from: &Address, amount: i128) {
    let client = get_sac_client(e);
    client.clawback(from, &amount);
}
