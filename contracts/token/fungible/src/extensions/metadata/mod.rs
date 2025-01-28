mod storage;
pub use self::storage::{
    decimals, get_metadata, name, set_metadata, symbol, Metadata, METADATA_KEY,
};

mod test;
