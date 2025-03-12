/// Unlike other extensions, the `capped` extension does not provide a separate
/// trait. This is because its methods are not intended to be used
/// independently, like `burn` or `mint`. Instead, the `capped` extension
/// modifies the business logic of the `mint` function to enforce a supply cap.
///
/// This module provides the following functions:
/// - `query_cap`: Returns the maximum token supply.
/// - `check_cap`: Verifies whether minting a specified `amount` would exceed
///   the cap.
mod storage;
pub use self::storage::{check_cap, query_cap, set_cap, CAP_KEY};
mod test;
