/// Extensions for the non-fungible token contract.
///
/// This module contains optional extensions that can be used to add
/// functionality to the base non-fungible token implementation.
///
/// Currently available extensions:
/// - `enumerable`: Provides methods to enumerate and list tokens in the contract
pub mod enumerable;

pub use enumerable::NonFungibleEnumerable;
