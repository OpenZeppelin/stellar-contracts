//! Fungibe BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific
//! accounts.

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, String};
use stellar_default_impl_macro::default_impl;
use stellar_fungible::{
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
    pub fn __constructor(e: &Env, admin: Address, initial_supply: i128) {
        Base::set_metadata(
            e,
            18,
            String::from_str(e, "BlockList Token"),
            String::from_str(e, "BLT"),
        );

        // Set the admin for the BlockList extension
        BlockList::set_admin(e, &admin);

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

    fn block_user(e: &Env, user: Address) {
        BlockList::block_user(e, &user)
    }

    fn unblock_user(e: &Env, user: Address) {
        BlockList::unblock_user(e, &user)
    }
}
