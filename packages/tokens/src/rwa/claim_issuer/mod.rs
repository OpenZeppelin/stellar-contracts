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

/// Trait for signature verification schemes.
///
/// Each signature scheme implements this trait to provide a consistent
/// interface for claim validation while allowing for scheme-specific
/// implementation details.
pub trait SignatureVerifier {
    /// The signature data type for this signature scheme.
    type SignatureData;

    /// Extracts and returns the parsed signature data from the raw signature bytes.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `sig_data` - The signature data to parse.
    ///
    /// # Errors
    ///
    /// * [`ClaimIssuerError::SigDataMismatch`] - If signature data format is
    ///   invalid.
    fn extract_signature_data(e: &Env, sig_data: &Bytes) -> Self::SignatureData;

    /// Validates a claim signature using the parsed signature data and returns true if valid.
    ///
    /// # Arguments
    ///
    /// * `e` - The Soroban environment.
    /// * `identity` - The identity address the claim is about.
    /// * `claim_topic` - The topic of the claim to validate.
    /// * `claim_data` - The claim data to validate.
    /// * `signature_data` - The parsed signature data.
    fn verify_claim_with_data(
        e: &Env,
        identity: &Address,
        claim_topic: u32,
        claim_data: &Bytes,
        signature_data: &Self::SignatureData,
    ) -> bool;

    /// Returns the expected signature data length for this scheme.
    fn expected_sig_data_len() -> u32;
}

// ################## ERRORS ##################

// TODO: correct enumeration and move up to higher level
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimIssuerError {
    SigDataMismatch = 1,
}
