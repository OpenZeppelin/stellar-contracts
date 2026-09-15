/// A compact enforcement event should remain far below the network's 16 KiB
/// transaction-wide event limit, even at the maximum supported signer count.
const ENFORCED_EVENT_SIZE_CEILING_BYTES: usize = 512;

mod enforced_context;
pub mod simple_threshold;
pub mod spending_limit;
pub mod weighted_threshold;
