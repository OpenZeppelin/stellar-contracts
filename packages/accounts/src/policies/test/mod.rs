/// Regression ceiling for the tested events, including `MAX_SIGNERS` and an
/// ExternalRef tag that fits a 250-byte ledger-key limit. This is not a bound
/// for arbitrary tags or the sum of all events in a transaction.
const ENFORCED_EVENT_SIZE_CEILING_BYTES: usize = 1024;

mod enforced_context;
pub mod simple_threshold;
pub mod spending_limit;
pub mod weighted_threshold;
