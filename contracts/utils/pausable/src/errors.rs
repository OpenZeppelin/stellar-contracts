use soroban_sdk::contracterror;

#[contracterror]
#[repr(u32)]
pub enum PausableError {
    /// The operation failed because the contract is paused.
    EnforcedPause = 1,
    /// The operation failed because the contract is not paused.
    ExpectedPause = 2,
}

