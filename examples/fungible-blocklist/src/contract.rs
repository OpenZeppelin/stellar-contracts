//! Fungibe BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific
//! accounts.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};
use stellar_access::{AccessControl, AccessController};
use stellar_macros::only_role;
use stellar_tokens::{BlockList, FungibleBlockList, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: &Address, manager: &Address, initial_supply: i128) {
        Self::set_metadata(
            e,
            18,
            String::from_str(e, "BlockList Token"),
            String::from_str(e, "BLT"),
        );

        Self::init_admin(e, admin);
        // create a role "manager" and grant it to `manager`
        Self::grant_role_no_auth(e, admin, manager, &symbol_short!("manager"));
        // Mint initial supply to the admin
        Self::internal_mint(e, admin, initial_supply);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type Impl = BlockList;
}

#[contractimpl]
impl AccessControl for ExampleContract {
    type Impl = AccessController;
}

#[contractimpl]
impl FungibleBlockList for ExampleContract {
    type Impl = BlockList;

    #[only_role(operator, "manager")]
    fn block_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::block_user(e, user, operator)
    }

    #[only_role(operator, "manager")]
    fn unblock_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::unblock_user(e, user, operator)
    }
}
