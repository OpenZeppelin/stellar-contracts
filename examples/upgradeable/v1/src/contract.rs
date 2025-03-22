use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Symbol,
};
use stellar_upgradeable::UpgradeableInternal;
use stellar_upgradeable_macros::Upgradeable;

pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

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
    fn _upgrade_auth(e: &Env, operator: &Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if *operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
    }
}
