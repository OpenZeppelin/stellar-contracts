use soroban_sdk::{
    contract, contractimpl, contractmeta, symbol_short, Address, BytesN, Env, Symbol,
};
use stellar_upgradeable::Upgradeable;

contractmeta!(key = "binver", val = "1.0.0");

pub const OWNER: Symbol = symbol_short!("OWNER");

// ROLLBACK ???

// ensure upgrade()
// set meta
//#[derive(Upgradeable)]
#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        e.storage().instance().set(&OWNER, &admin);
    }
}

#[contractimpl]
impl Upgradeable for ExampleContract {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>) {
        let owner: Address = e.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();

        e.deployer().update_current_contract_wasm(new_wasm_hash);
        //stellar_upgradeable::upgrade(e, new_wasm_hash);

        // 1. set flag
        // 2. update
        // upgradeable::upgrade(new_wasm_hash)
    }
}
