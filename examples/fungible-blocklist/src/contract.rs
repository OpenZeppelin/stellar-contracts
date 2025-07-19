//! Fungibe BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific
//! accounts.

use soroban_sdk::{
    contract, contracterror, contractimpl, derive_contract, symbol_short, Address, Env, String,
};
use stellar_access_control::AccessControl;
use stellar_access_control_macros::has_role;
use stellar_fungible::{FungibleBlockList, FungibleBlockListExt, FungibleToken};

#[contract]
#[derive_contract(
    AccessControl(default = MyBlockList),
    FungibleToken(ext = FungibleBlockListExt),
    FungibleBlockList(default = MyBlockList),
)]
pub struct ExampleContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, initial_supply: i128) {
        Self::set_metadata(
            e,
            18,
            String::from_str(e, "BlockList Token"),
            String::from_str(e, "BLT"),
        );

        Self::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`

        <Self as AccessControl>::grant_role(e, &admin, &manager, &symbol_short!("manager"));

        // Mint initial supply to the admin
        Self::internal_mint(e, &admin, initial_supply);
    }
}

pub struct MyBlockList;

impl AccessControl for MyBlockList {
    type Impl = AccessControl!();
}

impl FungibleBlockList for MyBlockList {
    type Impl = FungibleBlockList!();

    #[has_role(operator, "manager")]
    fn block_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::block_user(e, user, operator)
    }

    #[has_role(operator, "manager")]
    fn unblock_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::unblock_user(e, user, operator)
    }
}
