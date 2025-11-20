//! Fungibe BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific
//! accounts.

use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Env, String};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_role};
use stellar_tokens::fungible::{
    blocklist::{BlockList, FungibleBlockList},
    Base, FungibleToken,
};

#[contract]
pub struct ExampleContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

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

        // Mint initial supply to the admin
        Base::mint(e, &admin, initial_supply);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = BlockList;
}

#[contractimpl]
impl FungibleBlockList for ExampleContract {
    fn blocked(e: &Env, account: Address) -> bool {
        BlockList::blocked(e, &account)
    }

    #[only_role(operator, "manager")]
    fn block_user(e: &Env, user: Address, operator: Address) {
        BlockList::block_user(e, &user)
    }

    #[only_role(operator, "manager")]
    fn unblock_user(e: &Env, user: Address, operator: Address) {
        BlockList::unblock_user(e, &user)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}
