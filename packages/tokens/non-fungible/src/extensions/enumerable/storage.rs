use soroban_sdk::{contracttype, panic_with_error, Address, Env};

/// Storage key that maps to [`AllowanceData`]
#[contracttype]
pub struct OwnerTokensKey {
    pub owner: Address,
    pub index: u32,
}

/// Storage keys for the data associated with `FungibleToken`
#[contracttype]
pub enum StorageKey {
    TotalSupply,
    OwnerTokens(OwnerTokensKey),
    GlobalTokens(u32),
}

// ################## QUERY STATE ##################

/// Returns the total amount of tokens stored by the contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
fn total_supply(e: &Env) -> u32 {
    e.storage().instance().get(&StorageKey::TotalSupply).unwrap_or(0)
}

/// Returns the `token_id` owned by `owner` at a given `index` in the
/// owner's local list. Use along with
/// [`crate::NonFungibleToken::balance()`] to enumerate all of `owner`'s
/// tokens.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - Account of the token's owner.
/// * `index` - Index of the token in the owner's local list.
fn get_owner_token_id(e: &Env, owner: &Address, index: u32) -> u32 {
    let key = StorageKey::OwnerTokens(OwnerTokensKey { owner: owner.clone(), index });
    e.storage().persistent().get::<_, u32>(&key).unwrap()
}

/// Returns the `token_id` at a given `index` in the global token list.
/// Use along with [`NonFungibleEnumerable::total_supply()`] to enumerate
/// all the tokens in the contract.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `index` - Index of the token in the owner's local list.
///
/// # Notes
///
/// **IMPORTANT**: This function is only intended for non-sequential
/// `token_id`s. For sequential `token_id`s, no need to call a function,
/// the `token_id` itself acts as the global index.
fn get_token_id(e: &Env, index: u32) -> u32 {
    let key = StorageKey::GlobalTokens(index);
    e.storage().persistent().get::<_, u32>(&key).unwrap()
}

// ################## CHANGE STATE ##################

pub fn sequential_mint() {}

pub fn non_sequential_mint() {}

pub fn sequential_burn() {}

pub fn non_sequential_burn() {}

pub fn transfer() {}
