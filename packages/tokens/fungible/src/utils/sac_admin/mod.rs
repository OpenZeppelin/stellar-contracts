mod storage;

mod test;

use soroban_sdk::{Address, Env};

/// Mintable Trait for Fungible Token
///
/// The `FungibleMintable` trait extends the `FungibleToken` trait to provide
/// the capability to mint tokens. This trait is designed to be used in
/// conjunction with the `FungibleToken` trait.
///
/// Excluding the `mint` functionality from the
/// [`crate::fungible::FungibleToken`] trait is a deliberate design choice to
/// accommodate flexibility and customization for various smart contract use
/// cases.
pub trait SACAdmin {
    fn mint(e: Env, to: Address, amount: i128, operator: Address);

    fn set_admin(e: Env, new_admin: Address, operator: Address);

    fn set_authorized(e: Env, id: Address, authorize: bool, operator: Address);

    fn clawback(e: Env, from: Address, amount: i128, operator: Address);
}
