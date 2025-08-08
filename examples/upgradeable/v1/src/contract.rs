/// A basic contract that demonstrates the usage of the `Upgradeable` derive
/// macro. It only implements `UpgradeableInternal` and the derive macro do the
/// rest of the job. The goal is to upgrade this "v1" contract with the contract
/// in "v2".
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_contract_utils::Upgradeable;

#[contract]
#[derive(Upgradeable)]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        Self::set_owner(e, &admin);
    }
}
