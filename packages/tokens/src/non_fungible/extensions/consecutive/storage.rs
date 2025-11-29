use core::mem;

use soroban_sdk::{contracttype, panic_with_error, Address, Env, String, TryFromVal, Val, Vec};

#[cfg(not(feature = "certora"))]
use crate::non_fungible::{
    burnable::emit_burn,
    emit_transfer,
};

use crate::non_fungible::{
    sequential::{self as sequential},
    Base, ContractOverrides, NonFungibleTokenError, OWNERSHIP_EXTEND_AMOUNT,
    OWNERSHIP_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT, OWNER_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT,
    TOKEN_TTL_THRESHOLD,
};

pub struct Consecutive;

impl ContractOverrides for Consecutive {
    fn owner_of(e: &Env, token_id: u32) -> Address {
        Consecutive::owner_of(e, token_id)
    }

    fn token_uri(e: &Env, token_id: u32) -> String {
        Consecutive::token_uri(e, token_id)
    }

    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        Consecutive::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        Consecutive::transfer_from(e, spender, from, to, token_id);
    }

    fn approve(
        e: &Env,
        approver: &Address,
        approved: &Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        Consecutive::approve(e, approver, approved, token_id, live_until_ledger);
    }
}

/// For 32,000 total IDs with ITEM of type u32 and 100 items per bucket:
///
/// Bucket 0
/// ├── Item 0 → bits for Token IDs 0..31
/// ├── Item 1 → bits for Token IDs 32..63
/// ├── ...
/// ├── Item 99 → bits for Token IDs 3_168..3_199
///
/// Bucket 1...8
///
/// Bucket 9
/// ├── Item 0 → bits for Token IDs 28_800..28_831
/// ├── ...
/// ├── Item 99 → bits for Token IDs 31_968..31_999
///
/// Number of elements in a bucket
pub const ITEMS_IN_BUCKET: usize = 100;
/// Number of IDs per item, which corresponds to the number of bits for a given
/// value
pub const IDS_IN_ITEM: usize = mem::size_of::<u32>() * 8; // 32
/// Total number of IDs in the whole bucket
pub const IDS_IN_BUCKET: usize = ITEMS_IN_BUCKET * IDS_IN_ITEM; // 3,200
/// Max. amount of tokens allowed to be minted at once in
/// [`Consecutive::batch_mint`]
pub const MAX_TOKENS_IN_BATCH: usize = 32_000; // 10 buckets * 100 items * 32

/// Storage keys for the data associated with the consecutive extension of
/// `NonFungibleToken`
#[contracttype]
pub enum NFTConsecutiveStorageKey {
    Approval(u32),
    Owner(u32),
    OwnershipBucket(u32),
    BurnedToken(u32),
}

impl Consecutive {
    // ################## QUERY STATE ##################

    /// Returns the address of the owner of the given `token_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - Token ID as a number.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - Occurs if the provided
    ///   `token_id` does not exist.
    pub fn owner_of(e: &Env, token_id: u32) -> Address {
        let next_id = sequential::next_token_id(e);
        if next_id == 0 {
            panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
        }

        let last_token_id = next_id - 1;
        let key = NFTConsecutiveStorageKey::BurnedToken(token_id);
        let is_burned = Consecutive::get_persistent_entry(e, &key).unwrap_or(false);
        if is_burned || token_id > last_token_id {
            panic_with_error!(&e, NonFungibleTokenError::NonExistentToken);
        }

        let ids_in_bucket = IDS_IN_BUCKET as u32;
        // index of the bucket that contains token_id
        let bucket_index = token_id / ids_in_bucket;
        // position of the token_id within its bucket (0-based)
        let relative_id = token_id % ids_in_bucket;
        // index of the bucket that contains the last token_id
        let last_bucket_index = last_token_id / ids_in_bucket;

        (bucket_index..=last_bucket_index)
            // filter only existing buckets and return with their corresponding indexes
            .filter_map(|i| {
                Consecutive::get_persistent_entry(e, &NFTConsecutiveStorageKey::OwnershipBucket(i))
                    .map(|bucket| (i, bucket))
            })
            // scan for a set bit and maps it to an ID
            .find_map(|(i, bucket)| {
                // If we're in the starting bucket, begin search from the token's relative
                // position; otherwise, start from the beginning of the bucket.
                let from_id = if i == bucket_index { relative_id } else { 0 };
                find_bit_in_bucket(bucket, from_id)
                    .map(|pos_in_bucket| i * ids_in_bucket + pos_in_bucket)
            })
            .iter()
            .find_map(|id| {
                Consecutive::get_persistent_entry(e, &NFTConsecutiveStorageKey::Owner(*id))
            })
            .unwrap_or_else(|| panic_with_error!(&e, NonFungibleTokenError::NonExistentToken))
    }

    /// Returns the URI for a specific `token_id`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token_id` - The identifier of the token.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - Occurs if the provided
    ///   `token_id` does not exist (burned or more than max).
    /// * refer to [`Base::base_uri`] errors.
    pub fn token_uri(e: &Env, token_id: u32) -> String {
        let is_burned =
            Consecutive::get_persistent_entry(e, &NFTConsecutiveStorageKey::BurnedToken(token_id))
                .unwrap_or(false);
        if is_burned || token_id >= sequential::next_token_id(e) {
            panic_with_error!(e, NonFungibleTokenError::NonExistentToken);
        }

        let base_uri = Base::base_uri(e);
        Base::compose_uri_for_token(e, base_uri, token_id)
    }

    // ################## CHANGE STATE ##################

    /// Mints a batch of tokens with consecutive IDs and attributes them to
    /// `to`. This function does NOT handle authorization.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - The address of the recipient.
    /// * `amount` - The number of tokens to mint.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::InvalidAmount`] - If try to mint `0` or more
    ///   than `MAX_TOKENS_IN_BATCH`.
    /// * refer to [`Base::increase_balance`] errors.
    /// * refer to [`Consecutive::set_ownership_in_bucket`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["consecutive_mint", to: Address]`
    /// * data - `[from_token_id: u32, to_token_id: u32]`
    ///
    /// # Security Warning
    ///
    /// **IMPORTANT**: The function intentionally lacks authorization controls.
    /// You MUST invoke it only from the constructor or implement proper
    /// authorization in the calling function. For example:
    ///
    /// ```ignore,rust
    /// fn mint_batch(e: &Env, to: &Address, amount: u32) {
    ///     // 1. Verify admin has minting privileges (optional)
    ///     let admin = e.storage().instance().get(&ADMIN_KEY).unwrap();
    ///     admin.require_auth();
    ///
    ///     // 2. Only then call the actual mint function
    ///     Consecutive::batch_mint(e, &to, amount);
    /// }
    /// ```
    ///
    /// Failing to add proper authorization could allow anyone to mint tokens!
    pub fn batch_mint(e: &Env, to: &Address, amount: u32) -> u32 {
        if amount == 0 || amount > MAX_TOKENS_IN_BATCH as u32 {
            panic_with_error!(&e, NonFungibleTokenError::InvalidAmount);
        }

        let first_id = sequential::increment_token_id(e, amount);

        Base::increase_balance(e, to, amount);
        let last_id = first_id + amount - 1;

        Self::set_ownership_in_bucket(e, last_id);
        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(last_id), &to);
        #[cfg(not(feature = "certora"))]
        emit_consecutive_mint(e, to, first_id, last_id);

        // return the last minted id
        last_id
    }

    /// Destroys the token with `token_id` from `from`, ensuring ownership
    /// checks, and emits a `burn` event.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// Authorization for `from` is required.
    pub fn burn(e: &Env, from: &Address, token_id: u32) {
        from.require_auth();

        Consecutive::update(e, Some(from), None, token_id);
        #[cfg(not(feature = "certora"))]
        emit_burn(e, from, token_id);
    }

    /// Destroys the token with `token_id` from `from`, ensuring ownership
    /// and approval checks, and emits a `burn` event.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The account that is allowed to burn the token on behalf of
    ///   the owner.
    /// * `from` - The account whose token is destroyed.
    /// * `token_id` - The identifier of the token to burn.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::check_spender_approval`] errors.
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["burn", from: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// Authorization for `spender` is required.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, token_id: u32) {
        spender.require_auth();

        Base::check_spender_approval(e, spender, from, token_id);

        Consecutive::update(e, Some(from), None, token_id);
        #[cfg(not(feature = "certora"))]
        emit_burn(e, from, token_id);
    }

    /// Transfers a non-fungible token (NFT), ensuring ownership checks.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `from` - The current owner's address.
    /// * `to` - The recipient's address.
    /// * `token_id` - The identifier of the token being transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for `from` is required.
    /// * **IMPORTANT**: If the recipient is unable to receive, the NFT may get
    ///   lost.
    pub fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        from.require_auth();

        Consecutive::update(e, Some(from), Some(to), token_id);
        #[cfg(not(feature = "certora"))]
        emit_transfer(e, from, to, token_id);
    }

    /// Transfers a non-fungible token (NFT), ensuring ownership and approval
    /// checks.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `spender` - The address attempting to transfer the token.
    /// * `from` - The current owner's address.
    /// * `to` - The recipient's address.
    /// * `token_id` - The identifier of the token being transferred.
    ///
    /// # Errors
    ///
    /// * refer to [`Base::check_spender_approval`] errors.
    /// * refer to [`Consecutive::update`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[token_id: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for `spender` is required.
    /// * **IMPORTANT**: If the recipient is unable to receive, the NFT may get
    ///   lost.
    pub fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, token_id: u32) {
        spender.require_auth();

        Base::check_spender_approval(e, spender, from, token_id);

        Consecutive::update(e, Some(from), Some(to), token_id);
        #[cfg(not(feature = "certora"))]
        emit_transfer(e, from, to, token_id);
    }

    /// Approves an address to transfer a specific token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `approver` - The address of the approver (should be `owner` or
    ///   `operator`).
    /// * `approved` - The address receiving the approval.
    /// * `token_id` - The identifier of the token to be approved.
    /// * `live_until_ledger` - The ledger number at which the approval expires.
    ///
    /// # Errors
    ///
    /// * refer to [`Consecutive::owner_of`] errors.
    /// * refer to [`Base::approve_for_owner`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["approve", owner: Address, token_id: u32]`
    /// * data - `[approved: Address, live_until_ledger: u32]`
    ///
    /// # Notes
    ///
    /// * Authorization for `approver` is required.
    pub fn approve(
        e: &Env,
        approver: &Address,
        approved: &Address,
        token_id: u32,
        live_until_ledger: u32,
    ) {
        approver.require_auth();

        let owner = Consecutive::owner_of(e, token_id);
        Base::approve_for_owner(e, &owner, approver, approved, token_id, live_until_ledger);
    }

    /// Low-level function for handling transfers, mints and burns of an NFT,
    /// without handling authorization. Updates ownership records, adjusts
    /// balances, and clears existing approvals.
    ///
    /// The difference with [`Base::update`] is that the
    /// current function:
    /// 1. explicitly adds burned tokens to storage in
    ///    `NFTConsecutiveStorageKey::BurnedToken`,
    /// 2. sets the next token (if any) to the previous owner.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the current token owner.
    /// * `to` - The address of the token recipient.
    /// * `token_id` - The identifier of the token to be transferred.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::IncorrectOwner`] - If the `from` address is
    ///   not the owner of the token.
    /// * refer to [`Consecutive::owner_of`] errors.
    /// * refer to [`Base::decrease_balance`] errors.
    /// * refer to [`Base::increase_balance`] errors.
    /// * refer to [`Consecutive::set_ownership_in_bucket`] errors.
    pub fn update(e: &Env, from: Option<&Address>, to: Option<&Address>, token_id: u32) {
        if let Some(from_address) = from {
            let owner = Consecutive::owner_of(e, token_id);

            // Ensure the `from` address is indeed the owner.
            if owner != *from_address {
                panic_with_error!(e, NonFungibleTokenError::IncorrectOwner);
            }

            Base::decrease_balance(e, from_address, 1);

            // Clear any existing approval
            let approval_key = NFTConsecutiveStorageKey::Approval(token_id);
            e.storage().temporary().remove(&approval_key);

            // Set the token_id - 1 to previous owner to preserve the ownership inference.
            // `set_owner_for_previous_token` does this, but will skip it if the previous id
            // doesn't exist, was burned or has already an owner.
            Consecutive::set_owner_for_previous_token(e, from_address, token_id);
        } else {
            // nothing to do for the `None` case, since we don't track
            // `total_supply`
        }

        if let Some(to_address) = to {
            Base::increase_balance(e, to_address, 1);

            // Set the new owner
            e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(token_id), to_address);
            Self::set_ownership_in_bucket(e, token_id);
        } else {
            // Burning: `to` is None
            e.storage().persistent().remove(&NFTConsecutiveStorageKey::Owner(token_id));

            e.storage().persistent().set(&NFTConsecutiveStorageKey::BurnedToken(token_id), &true);
        }
    }

    /// Low-level function that sets owner of `token_id - 1` to `to`, without
    /// handling authorization. The function does not panic and sets the
    /// owner only if:
    /// - the token exists and
    /// - the token has not been burned and
    /// - the token doesn't have an owner.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `to` - The owner's address.
    /// * `token_id` - The identifier of the token next to the one being set.
    ///
    /// # Notes
    ///
    /// This function extends the persistent storage TTL even when it doesn't
    /// assign an owner. The intent is to fairly distribute storage costs among
    /// neighboring entries, since they collectively influence boundary
    /// calculations.
    pub fn set_owner_for_previous_token(e: &Env, to: &Address, token_id: u32) {
        if token_id == 0 || token_id >= sequential::next_token_id(e) {
            return;
        }
        let previous_id = token_id - 1;

        let key = NFTConsecutiveStorageKey::Owner(previous_id);
        let has_owner = Consecutive::get_persistent_entry::<Address>(e, &key).is_some();
        if has_owner {
            return;
        }

        let key = NFTConsecutiveStorageKey::BurnedToken(previous_id);
        let is_burned = Consecutive::get_persistent_entry::<bool>(e, &key).unwrap_or(false);
        if is_burned {
            return;
        }

        e.storage().persistent().set(&NFTConsecutiveStorageKey::Owner(previous_id), to);
        Self::set_ownership_in_bucket(e, previous_id);
    }

    /// Low-level function that marks `token_id` as being owned, i.e. sets the
    /// corresponding bit in the ownership bucket.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `token_id` - The identifier of the token being set.
    ///
    /// # Errors
    ///
    /// * [`NonFungibleTokenError::NonExistentToken`] - If `token_id` does not
    ///   exist yet (has not been minted).
    pub fn set_ownership_in_bucket(e: &Env, token_id: u32) {
        let ids_in_bucket = IDS_IN_BUCKET as u32;
        let ids_in_item = IDS_IN_ITEM as u32;

        if token_id >= sequential::next_token_id(e) {
            panic_with_error!(e, NonFungibleTokenError::NonExistentToken);
        }

        let bucket_index = token_id / ids_in_bucket;

        let key = NFTConsecutiveStorageKey::OwnershipBucket(bucket_index);
        let mut bucket: Vec<u32> = if let Some(b) = Consecutive::get_persistent_entry(e, &key) {
            b
        } else {
            Vec::from_slice(e, &[0; ITEMS_IN_BUCKET])
        };

        // position of the token_id within its bucket (0-based)
        let relative_id = token_id % ids_in_bucket;
        // index of the item inside the bucket that contains token_id
        let item_index = relative_id / ids_in_item;
        // index of the bit within the item that contains token_id
        let bit_index = relative_id % ids_in_item;

        let mask: u32 = 1 << (ids_in_item - bit_index - 1);
        let mut item = bucket.get(item_index).expect("token_id out of allowed range");

        // return early if the bit was already set in a previous action (transfer, burn
        // or batch_mint)
        if item & mask != 0 {
            return;
        }

        item |= mask;
        bucket.set(item_index, item);

        e.storage()
            .persistent()
            .set(&NFTConsecutiveStorageKey::OwnershipBucket(bucket_index), &bucket);
    }

    /// Low-level function that tries to retrieve a persistent storage value and
    /// extend its TTL if the entry exists.
    ///
    /// # Arguments
    ///
    /// * `e` - The environment reference.
    /// * `key` - The key required to retrieve the underlying storage.
    fn get_persistent_entry<T: TryFromVal<Env, Val>>(
        e: &Env,
        key: &NFTConsecutiveStorageKey,
    ) -> Option<T> {
        e.storage().persistent().get::<_, T>(key).inspect(|_| {
            use NFTConsecutiveStorageKey::*;

            let const_vals = match key {
                BurnedToken(_) => [TOKEN_TTL_THRESHOLD, TOKEN_EXTEND_AMOUNT],
                Owner(_) => [OWNER_TTL_THRESHOLD, OWNER_EXTEND_AMOUNT],
                OwnershipBucket(_) => [OWNERSHIP_TTL_THRESHOLD, OWNERSHIP_EXTEND_AMOUNT],
                Approval(_) => panic!("Approval is in temporary storage"),
            };
            e.storage().persistent().extend_ttl(key, const_vals[0], const_vals[1]);
        })
    }
}

/// Searches for the first set bit (`1`) in a bitfield, scanning from left to
/// right, starting at the specified bit position (relative to the most
/// significant bit).
///
/// # Arguments
///
/// * `input` - An optional bitfield (`u32`) to search within.
/// * `start` - Bit index to start the search from, counted from the MSB
///   (0-based).
///
/// # Returns
///
/// * `Some(index)` - The index (relative to the MSB) of the first set bit
///   found, starting from `start` toward the LSB.
/// * `None` - If `input` is `None`, `start` is out of range, or no set bits are
///   found.
///
/// # Example
///
/// If `u8::BITS = 8` and `input = Some(0b00010100)`:
/// - `find_bit_in_item(input, 2)` returns `Some(3)` because the 4th set bit
///   (from MSB) is at index 3 (counting from MSB = 0).
pub(crate) fn find_bit_in_item(input: Option<u32>, start: u32) -> Option<u32> {
    if let Some(num) = input {
        // return early if 0 (no bits are set)
        if num == 0 {
            return None;
        }

        let ids_in_item = u32::BITS;
        // Invalid start position
        if start >= ids_in_item {
            return None;
        }

        let last = ids_in_item - 1;

        for i in (0..=(last - start)).rev() {
            if (num & (1 << i)) != 0 {
                // i goes from MSB toward LSB relative to `start`, but we want to return
                // MSB-relative index
                return Some(last - i);
            }
        }
    }

    None
}

/// Searches for the first set bit (`1`) in a vector of bitfields ("bucket"),
/// starting at a given bit index relative to the entire bucket (from MSB).
///
/// Internally, each element in the `bucket` represents a smaller bitfield (an
/// "item"). This function locates which item to begin with, and then searches
/// for a set bit across subsequent items using `find_bit_in_item`.
///
/// # Arguments
///
/// * `bucket` - A vector of `u32`s, where each value is treated as a bitfield.
/// * `start` - The starting bit index to search from, relative to the MSB of
///   the whole bucket.
///
/// # Returns
///
/// * `Some(index)` - The bit index (relative to the entire bucket) of the first
///   set bit found.
/// * `None` - If `start` is out of range, or no set bit is found from that
///   position onwards.
///
/// # Example
///
/// If `IDS_IN_ITEM = 8`, `ITEMS_IN_BUCKET = 2`, and:
/// ```
/// bucket = vec![0b00000000, 0b00101000];
/// ```
/// then `find_bit_in_bucket(bucket, 8)` returns `Some(10)`, since bit 3 of the
/// second item (index 10 in the overall bucket) is the first set bit.
pub(crate) fn find_bit_in_bucket(bucket: Vec<u32>, start: u32) -> Option<u32> {
    let ids_in_item = u32::BITS;
    let ids_in_bucket = bucket.len() * ids_in_item;

    // Invalid start position
    if start >= ids_in_bucket {
        return None;
    }

    let item_index = start / ids_in_item;
    // relative ID in item
    let relative_id = start % ids_in_item;

    (item_index..bucket.len()).find_map(|i| {
        // If we're in the starting item, begin search from the token's relative
        // position; otherwise, start from the beginning of the item.
        let from_id = if i == item_index { relative_id } else { 0 };

        find_bit_in_item(bucket.get(i), from_id).map(|pos_in_item| i * ids_in_item + pos_in_item)
    })
}
