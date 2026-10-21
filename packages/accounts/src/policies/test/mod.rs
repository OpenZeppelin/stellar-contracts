/// Regression ceiling for the tested events, including `MAX_SIGNERS` and an
/// ExternalRef tag that fits a 250-byte ledger-key limit. This is not a bound
/// for arbitrary tags or the sum of all events in a transaction.
const ENFORCED_EVENT_SIZE_CEILING_BYTES: usize = 1024;

/// `txMaxContractEventsSizeBytes` as configured on testnet under Protocol 27
/// (`ConfigSetting(contract_events_v0)`), where the overflow behind #852 was
/// measured. A network setting rather than a protocol constant, so it can
/// change; the tests only need a value that a 20 KB tag is sure to exceed.
const TX_MAX_CONTRACT_EVENTS_SIZE_BYTES: usize = 16_384;

mod enforced_context;
pub mod simple_threshold;
pub mod spending_limit;
pub mod weighted_threshold;
