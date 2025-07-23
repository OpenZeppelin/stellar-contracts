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
//! - **Minting**: `batch_mint` stores the owner only for the last token ID in
//!   the batch.
//!
//!   Mint first 10 tokens to A
//!   -------------------A
//!   |0|1|2|3|4|5|6|7|8|9|
//!
//! - **owner_of**: Walks upwards from the token ID to find the closest recorded
//!   owner.
//!
//!   `owner_of(4) == A`
//!   --------->---------A
//!   |0|1|2|3|4|5|6|7|8|9|
//!
//! - **Transfer**: Stores the new owner for the token ID and re-stores the old
//!   owner at `token_id - 1` if needed, to preserve correct inference for
//!   previous tokens.
//!
//!   After transfer of token 5 to B
//!   ---------A-B-------A
//!   |0|1|2|3|4|5|6|7|8|9|
//!
//! - **Burn**: Removes the owner, marks the token as burnt, and (if needed)
//!   stores the old owner at `token_id - 1`.
//!
//!   Burn token 2
//!   ---A-x---A-B-------A
//!   |0|1|2|3|4|5|6|7|8|9|
//!
//!
//! ## Caveats
//!
//! - Slightly more expensive reads due to reverse scan in `owner_of`. Please
//!   note that after Protocol 23 the cost of storage reads will be marginal, so
//!   the overhead of this approach will be minimal.
//! - Requires extra logic to preserve ownership inference when transferring or
//!   burning tokens.
//! - To avoid exceeding resource read limits per transaction (especially during
//!   `owner_of` calls), a maximum number of tokens is enforced per
//!   `batch_mint`. The rationale is that the gaps between set bits within
//!   buckets can only shrink over time, ensuring that future lookups remain
//!   within the bounds defined by `batch_mint`.
//!
//! ## Usage
//!
//! - It is not recommended to use this model if each token is expected to be
//!   minted separately. It is rather best suited for NFTs where minting happens
//!   in large batches.
//! - **IMPORTANT**: For minting tokens ONLY the function `batch_mint` provided
//!   in this extension must be used. Using other minting functions will break
//!   the logic of tracking ownership.
pub mod storage;
use soroban_sdk::{Address, Env, Symbol};
pub use storage::Consecutive;


mod test;

// ################## EVENTS ##################

/// Emits an event indicating a mint of a batch of tokens.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `to` - The address receiving the new tokens.
/// * `from_token_id` - First token ID in the batch.
/// * `to_token_id` - Last token ID of the batch.
///
/// # Events
///
/// * topics - `["consecutive_mint", to: Address]`
/// * data - `[from_token_id: u32, to_token_id: u32]`
pub fn emit_consecutive_mint(e: &Env, to: &Address, from_token_id: u32, to_token_id: u32) {
    let topics = (Symbol::new(e, "consecutive_mint"), to);
    e.events().publish(topics, (from_token_id, to_token_id))
}
