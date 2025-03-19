use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};
use stellar_upgradeable::UpgradeableInternal;
use stellar_upgradeable_macros::Upgradeable;

pub const OWNER: Symbol = symbol_short!("OWNER");

#[derive(Upgradeable)]
#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        e.storage().instance().set(&OWNER, &admin);
    }
}

impl UpgradeableInternal for ExampleContract {
    type UpgradeData = ();

    fn _upgrade(e: &Env, _data: &Self::UpgradeData) {
        let owner: Address = e.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();
    }
}
