/// Unlike other extensions, the `capped` extension does not provide a separate
/// trait. This is because its methods are not intended to be used
/// independently, like [`crate::extensions::burnable::burn()`].
/// Instead, the `capped` extension modifies the business logic of the `mint`
/// function to enforce a supply cap.
///
/// This module provides the following helper functions:
/// - `set_cap`: Sets the maximum token supply.
/// - `query_cap`: Returns the maximum token supply.
/// - `check_cap`: Panics if minting a specified `amount` would exceed the cap.
///   Should be used before calling `mint()`.
mod storage;
pub use self::storage::{check_cap, query_cap, set_cap, CAP_KEY};
mod test;
