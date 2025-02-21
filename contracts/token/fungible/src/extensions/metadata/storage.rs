use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env, String, Symbol};

/// Storage key that maps to [`Metadata`]
pub const METADATA_KEY: Symbol = symbol_short!("METADATA");

/// Storage container for token metadata
#[contracttype]
pub struct Metadata {
    pub decimals: u32,
    pub name: String,
    pub symbol: String,
}

/// Returns the token metadata such as decimals, name and symbol.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::UnsetMetadata`] - When trying to access
///   uninitialized metadata.
pub fn get_metadata(e: &Env) -> Metadata {
    e.storage()
        .instance()
        .get(&METADATA_KEY)
        .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::UnsetMetadata))
}

/// Returns the token decimals.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::UnsetMetadata`] - When trying to access
///   uninitialized metadata.
pub fn decimals(e: &Env) -> u32 {
    get_metadata(e).decimals
}

/// Returns the token name.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::UnsetMetadata`] - When trying to access
///   uninitialized metadata.
pub fn name(e: &Env) -> String {
    get_metadata(e).name
}

/// Returns the token symbol.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`FungibleTokenError::UnsetMetadata`] - When trying to access
///   uninitialized metadata.
pub fn symbol(e: &Env) -> String {
    get_metadata(e).symbol
}

/// Sets the token metadata such as decimals, name and symbol.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `decimals` - The number of decimals.
/// * `name` - The name of the token.
/// * `symbol` - The symbol of the token.
///
/// # Notes
///
/// **IMPORTANT**: This function lacks authorization controls. You want to
/// invoke it most likely from a constructor or from another function with
/// admin-only authorization.
pub fn set_metadata(e: &Env, decimals: u32, name: String, symbol: String) {
    let metadata = Metadata { decimals, name, symbol };
    e.storage().instance().set(&METADATA_KEY, &metadata);
}
