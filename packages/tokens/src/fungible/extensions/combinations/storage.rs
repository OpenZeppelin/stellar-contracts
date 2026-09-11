use soroban_sdk::{panic_with_error, Address, Env, MuxedAddress};
use stellar_governance::votes::transfer_voting_units;

use crate::fungible::{
    extensions::{
        allowlist::{AllowList, AllowListContractType},
        blocklist::{BlockList, BlockListContractType},
        total_supply::{decrease_total_supply, mint, total_supply, TotalSupplyOverrides},
        votes::FungibleVotes,
    },
    overrides::{BurnableOverrides, ContractOverrides},
    FungibleTokenError,
};

/// Contract type combining the [`AllowList`] and [`BlockList`] transfer
/// policies: an account has to be allowed and must not be blocked to send,
/// receive, approve or burn tokens.
///
/// The blocklist is checked first, so an account that is both blocked and not
/// allowed fails with [`FungibleTokenError::UserBlocked`].
pub struct AllowBlockList;

// The combined contract type keeps backing both list extensions.
impl AllowListContractType for AllowBlockList {}
impl BlockListContractType for AllowBlockList {}

impl ContractOverrides for AllowBlockList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowBlockList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowBlockList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowBlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for AllowBlockList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        AllowBlockList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowBlockList::burn_from(e, spender, from, amount);
    }
}

impl AllowBlockList {
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, &to.address()) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::transfer(e, from, to, amount);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::transfer_from(e, spender, from, to, amount);
    }

    pub fn approve(
        e: &Env,
        owner: &Address,
        spender: &Address,
        amount: i128,
        live_until_ledger: u32,
    ) {
        if BlockList::blocked(e, owner) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::approve(e, owner, spender, amount, live_until_ledger);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::burn(e, from, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::burn_from(e, spender, from, amount);
    }
}

/// Contract type combining the [`AllowList`] transfer policy with vote
/// checkpoints: every mint, transfer and burn goes through the list policy and
/// moves the corresponding voting units.
pub struct AllowListVotes;

impl ContractOverrides for AllowListVotes {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowListVotes::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowListVotes::transfer_from(e, spender, from, to, amount);
    }

    // Approvals do not move balances, so the list policy applies unchanged.
    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for AllowListVotes {
    fn burn(e: &Env, from: &Address, amount: i128) {
        AllowListVotes::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowListVotes::burn_from(e, spender, from, amount);
    }
}

impl AllowListVotes {
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowList::burn_from(e, spender, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }
}

/// Contract type combining the [`BlockList`] transfer policy with vote
/// checkpoints: every mint, transfer and burn goes through the list policy and
/// moves the corresponding voting units.
pub struct BlockListVotes;

impl ContractOverrides for BlockListVotes {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        BlockListVotes::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockListVotes::transfer_from(e, spender, from, to, amount);
    }

    // Approvals do not move balances, so the list policy applies unchanged.
    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        BlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for BlockListVotes {
    fn burn(e: &Env, from: &Address, amount: i128) {
        BlockListVotes::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        BlockListVotes::burn_from(e, spender, from, amount);
    }
}

impl BlockListVotes {
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        BlockList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        BlockList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        BlockList::burn_from(e, spender, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }
}

/// Contract type combining both list policies ([`AllowBlockList`]) with vote
/// checkpoints: every mint, transfer and burn goes through the list policy and
/// moves the corresponding voting units.
pub struct AllowBlockListVotes;

impl ContractOverrides for AllowBlockListVotes {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowBlockListVotes::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowBlockListVotes::transfer_from(e, spender, from, to, amount);
    }

    // Approvals do not move balances, so the list policy applies unchanged.
    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowBlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for AllowBlockListVotes {
    fn burn(e: &Env, from: &Address, amount: i128) {
        AllowBlockListVotes::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowBlockListVotes::burn_from(e, spender, from, amount);
    }
}

impl AllowBlockListVotes {
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowBlockList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowBlockList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowBlockList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowBlockList::burn_from(e, spender, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }
}

// The combined contract types keep backing their list extensions.
impl AllowListContractType for AllowListVotes {}
impl BlockListContractType for BlockListVotes {}
impl AllowListContractType for AllowBlockListVotes {}
impl BlockListContractType for AllowBlockListVotes {}

// Voting units mirror balances; a zero amount moves nothing, matching
// [`FungibleVotes`].
fn move_voting_units(e: &Env, from: Option<&Address>, to: Option<&Address>, amount: i128) {
    if amount > 0 {
        transfer_voting_units(e, from, to, amount as u128);
    }
}

/// Contract type combining the [`AllowList`] transfer policy with total
/// supply tracking.
pub struct TotalSupplyAllowList;

/// Contract type combining the [`BlockList`] transfer policy with total
/// supply tracking.
pub struct TotalSupplyBlockList;

impl TotalSupplyOverrides for TotalSupplyAllowList {}
impl TotalSupplyOverrides for TotalSupplyBlockList {}

// The combined contract types keep enforcing their respective list policy.
impl AllowListContractType for TotalSupplyAllowList {}
impl BlockListContractType for TotalSupplyBlockList {}

// Transfers and approvals never touch the total supply, so they are routed
// to the respective list policy unchanged.
impl ContractOverrides for TotalSupplyAllowList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl ContractOverrides for TotalSupplyBlockList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        BlockList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        BlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for TotalSupplyAllowList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupplyAllowList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupplyAllowList::burn_from(e, spender, from, amount);
    }
}

impl BurnableOverrides for TotalSupplyBlockList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupplyBlockList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupplyBlockList::burn_from(e, spender, from, amount);
    }
}

impl TotalSupplyAllowList {
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowList::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowList::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}

impl TotalSupplyBlockList {
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        BlockList::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        BlockList::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}

/// Contract type combining both list policies ([`AllowBlockList`]) with total
/// supply tracking.
pub struct TotalSupplyAllowBlockList;

impl TotalSupplyOverrides for TotalSupplyAllowBlockList {}

// The combined contract type keeps enforcing both list policies.
impl AllowListContractType for TotalSupplyAllowBlockList {}
impl BlockListContractType for TotalSupplyAllowBlockList {}

// Transfers and approvals never touch the total supply, so they are routed
// to the list policies unchanged.
impl ContractOverrides for TotalSupplyAllowBlockList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowBlockList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowBlockList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowBlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for TotalSupplyAllowBlockList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupplyAllowBlockList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupplyAllowBlockList::burn_from(e, spender, from, amount);
    }
}

impl TotalSupplyAllowBlockList {
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowBlockList::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowBlockList::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}
