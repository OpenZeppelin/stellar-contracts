use soroban_sdk::{panic_with_error, Address, Env, MuxedAddress};
use stellar_governance::votes::transfer_voting_units;

use crate::fungible::{
    extensions::{
        allowlist::{AllowList, AllowListContractType},
        blocklist::{BlockList, BlockListContractType},
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
    /// Transfers `amount` of tokens from `from` to `to`, applying both list
    /// policies.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When either `from` or `to` is
    ///   blocked.
    /// * refer to [`AllowList::transfer`] errors.
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, &to.address()) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::transfer(e, from, to, amount);
    }

    /// Transfers `amount` of tokens from `from` to `to` using the allowance
    /// mechanism, applying both list policies.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When either `from` or `to` is
    ///   blocked.
    /// * refer to [`AllowList::transfer_from`] errors.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        if BlockList::blocked(e, from) || BlockList::blocked(e, to) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::transfer_from(e, spender, from, to, amount);
    }

    /// Sets the allowance of `spender` over the tokens of `owner`, applying
    /// both list policies to `owner`.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When `owner` is blocked.
    /// * refer to [`AllowList::approve`] errors.
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

    /// Destroys `amount` of tokens from `from`, applying both list policies.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When `from` is blocked.
    /// * refer to [`AllowList::burn`] errors.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        if BlockList::blocked(e, from) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }
        AllowList::burn(e, from, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism,
    /// applying both list policies.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::UserBlocked`] - When `from` is blocked.
    /// * refer to [`AllowList::burn_from`] errors.
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
    /// refer to [`AllowList::transfer`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    /// refer to [`AllowList::transfer_from`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    /// Minting is not gated by the list policy.
    ///
    /// refer to [`FungibleVotes::mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    /// refer to [`AllowList::burn`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    /// refer to [`AllowList::burn_from`] and [`transfer_voting_units`] for the
    /// inline documentation.
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
    /// refer to [`BlockList::transfer`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        BlockList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    /// refer to [`BlockList::transfer_from`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    /// Minting is not gated by the list policy.
    ///
    /// refer to [`FungibleVotes::mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    /// refer to [`BlockList::burn`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        BlockList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    /// refer to [`BlockList::burn_from`] and [`transfer_voting_units`] for the
    /// inline documentation.
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
    /// refer to [`AllowBlockList::transfer`] and [`transfer_voting_units`] for
    /// the inline documentation.
    pub fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowBlockList::transfer(e, from, to, amount);
        move_voting_units(e, Some(from), Some(&to.address()), amount);
    }

    /// refer to [`AllowBlockList::transfer_from`] and [`transfer_voting_units`]
    /// for the inline documentation.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowBlockList::transfer_from(e, spender, from, to, amount);
        move_voting_units(e, Some(from), Some(to), amount);
    }

    /// Minting is not gated by the list policy.
    ///
    /// refer to [`FungibleVotes::mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        FungibleVotes::mint(e, to, amount);
    }

    /// refer to [`AllowBlockList::burn`] and [`transfer_voting_units`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowBlockList::burn(e, from, amount);
        move_voting_units(e, Some(from), None, amount);
    }

    /// refer to [`AllowBlockList::burn_from`] and [`transfer_voting_units`] for
    /// the inline documentation.
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
