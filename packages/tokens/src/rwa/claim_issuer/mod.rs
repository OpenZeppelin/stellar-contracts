mod storage;
mod test;

pub use storage::*;

use soroban_sdk::{contractclient, contracterror, Address, Bytes, Env};

/// Trait for validating claims issued by this identity to other identities.
#[contractclient(name = "ClaimIssuerClient")]
pub trait ClaimIssuer {
    /// Validates whether a claim is valid for a given identity.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `sig_data` - The signature data.
    /// * `claim_data` - The claim data to validate.
    ///
    /// # Returns
    ///
    /// Returns true if the claim is valid, false otherwise.
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) -> bool;
}

// ################## ERRORS ##################

// TODO: correct enumeration and move up to higher level
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimIssuerError {
    SigDataMismatch = 1,
}
