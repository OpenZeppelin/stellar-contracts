//! Fungible AllowList Example Contract.

//! This contract showcases how to integrate the AllowList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can allow or disallow specific
//! accounts.

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Env, MuxedAddress, String, Symbol, Vec,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::only_role;
use stellar_tokens::fungible::{
    allowlist::{AllowList, FungibleAllowList},
    burnable::FungibleBurnable,
    Base, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(
        e: &Env,
        name: String,
        symbol: String,
        admin: Address,
        manager: Address,
        initial_supply: i128,
    ) {
        Base::set_metadata(e, 18, name, symbol);

        access_control::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &manager, &symbol_short!("manager"), &admin);

        // Allow the admin to transfer tokens
        AllowList::allow_user(e, &admin);

        // Mint initial supply to the admin
        Base::mint(e, &admin, initial_supply);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = AllowList;
}
#[contractimpl]
impl FungibleAllowList for ExampleContract {
    fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }

    #[only_role(operator, "manager")]
    fn allow_user(e: &Env, user: Address, operator: Address) {
        AllowList::allow_user(e, &user)
    }

    #[only_role(operator, "manager")]
    fn disallow_user(e: &Env, user: Address, operator: Address) {
        AllowList::disallow_user(e, &user)
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ExampleContract {}

#[contractimpl(contracttrait)]
impl FungibleBurnable for ExampleContract {}
