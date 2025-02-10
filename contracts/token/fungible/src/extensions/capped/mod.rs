mod storage;
// pub use self::storage::{burn, burn_from};

// mod test;

use soroban_sdk::{contractclient, symbol_short, Address, Env};

/// Capped Trait for Fungible Token
///
/// The `Capped` trait extends the `FungibleToken` trait to provide the
/// capability to set a limit for total supply.
#[contractclient(name = "FungibleCappedClient")]
pub trait FungibleCapped {}
