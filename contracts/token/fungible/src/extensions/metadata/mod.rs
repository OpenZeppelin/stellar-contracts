/// Unlike other extensions, `metadata` does not provide a separate trait  
/// because the corresponding functions are already available in
/// [`crate::FungibleToken`].  
///
/// The decision to keep `metadata` as a standalone extension allows developers
/// the flexibility to either use dynamic metadata functions or hardcode  
/// values for `decimals`, `symbol`, and `name` when designing their token
/// contract.
mod storage;
pub use self::storage::{
    decimals, get_metadata, name, set_metadata, symbol, Metadata, METADATA_KEY,
};

mod test;
