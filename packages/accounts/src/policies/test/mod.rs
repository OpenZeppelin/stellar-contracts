/// Regression ceiling for a single enforcement event, including `MAX_SIGNERS`
/// signer IDs. This is not a bound for the sum of all events in a transaction.
const ENFORCED_EVENT_SIZE_CEILING_BYTES: usize = 1024;

mod enforced_context;
pub mod simple_threshold;
pub mod spending_limit;
pub mod weighted_threshold;
