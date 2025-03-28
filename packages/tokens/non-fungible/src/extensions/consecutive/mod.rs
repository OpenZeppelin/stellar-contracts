//! # Consecutive Extension for Non-Fungible Token
//!
//! The `consecutive` module provides an implementation for managing
//! NFTs by using consecutive token ownership tracking. This design is
//! inspired by ERC-721A and similar approaches that drastically reduces storage
//! writes during minting. Instead of recording the owner for every individual
//! token ID, the consecutive model stores ownership only at boundaries, and
//! infers ownership for other tokens based on the most recent known owner
//! before the given token ID.
//!
//! ## Implementation Notes
//!
//! - **Minting**: `batch_mint` stores the owner only for the first token ID in
//!   the batch.
//! - **owner_of**: Walks backwards from the token ID to find the closest
//!   recorded owner.
//! - **Transfer**: Stores the new owner for the token ID and re-stores the old
//!   owner at `token_id + 1` if needed, to preserve correct inference for later
//!   tokens.
//! - **Burn**: Removes the owner, marks the token as burnt, and (if needed)
//!   stores the old owner at `token_id + 1`.
//!
//! ## Caveats
//!
//! - Slightly more expensive reads due to reverse scan in `owner_of`. Please
//!   note that after Protocol 23 the cost of storage reads will be marginal, so
//!   the overhead of this approach will be minimal.
//! - Requires extra logic to preserve ownership inference when transferring or
//!   burning tokens.
//!
//! ## Usage
//!
//! - It is not recommended to use this model if each token is expected to be
//!   minted separately. It is rather best suited for NFTs where minting happens
//!   in large batches.
//! - **IMPORTANT** - For minting tokens ONLY the function `batch_mint` provided
//!   in this extension must be used. Using other minting functions will break
//!   the logic of tracking ownership.
pub mod overrides;
pub mod storage;
use overrides::Consecutive;
use soroban_sdk::{Address, Env, Symbol};

use crate::{NonFungibleToken, TokenId};

/// Consecutive Marker Trait for Non-Fungible Token
///
/// # Notes
///
/// The `consecutive` extension provides its own business logic for creating and
/// destroying tokens. Therefore, this trait is INCOMPATIBLE with the
/// `Mintable`, `Burnable`, and `Enumerable` extensions.
pub trait NonFungibleConsecutive: NonFungibleToken<ContractType = Consecutive> {}

mod test;

// ################## EVENTS ##################

/// Emits an event indicating a mint of a batch of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new token.
/// * `from_token_id` - First token id in the batch.
/// * `to_token_id` - Last token id of the batch.
///
/// # Events
///
/// * topics - `["consecutive_mint", to: Address]`
/// * data - `[from_token_id: TokenId, to_token_id: TokenId]`
pub fn emit_consecutive_mint(e: &Env, to: &Address, from_token_id: TokenId, to_token_id: TokenId) {
    let topics = (Symbol::new(e, "consecutive_mint"), to);
    e.events().publish(topics, (from_token_id, to_token_id))
}
