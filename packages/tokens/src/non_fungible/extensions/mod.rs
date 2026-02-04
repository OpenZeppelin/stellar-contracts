pub mod burnable;
pub mod consecutive;
pub mod enumerable;
pub mod royalties;

// Negative trait bounds
use crate::non_fungible::{consecutive::NonFungibleConsecutive, enumerable::NonFungibleEnumerable};

impl<T: NonFungibleEnumerable> !NonFungibleConsecutive for T {}
