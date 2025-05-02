use soroban_sdk::{contract, contractimpl, contracttype, token::StellarAssetClient, Address, Env};

#[contracttype]
pub enum SacDataKey {
    SAC,
    Owner,
}

#[contract]
pub struct SacAdmin;

#[contractimpl]
impl SacAdmin {
    pub fn __constructor(e: Env, sac: Address, owner: Address) {
        e.storage().instance().set(&SacDataKey::SAC, &sac);
        e.storage().instance().set(&SacDataKey::Owner, &owner);
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        // access control: only owner can call mint
        let owner: Address = e.storage().instance().get(&SacDataKey::Owner).unwrap();
        owner.require_auth();

        // mint on sac
        let sac_address: Address = e.storage().instance().get(&SacDataKey::SAC).unwrap();
        let token_client = StellarAssetClient::new(&e, &sac_address);
        token_client.mint(&to, &amount);
    }

    pub fn set_admin(_e: Env, _new_admin: Address) {
        todo!()
    }

    pub fn set_authorized(_e: Env, _id: Address, _authorize: bool) {
        todo!()
    }

    pub fn clawback(_e: Env, _from: Address, _amount: i128) {
        todo!()
    }
}
