pub mod allowlist;
pub mod blocklist;
pub mod burnable;
pub mod capped;

// disable allowlist and blocklist together
use crate::fungible::extensions::{allowlist::FungibleAllowList, blocklist::FungibleBlockList};

impl<T: FungibleAllowList> !FungibleBlockList for T {}
