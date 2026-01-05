use soroban_sdk::{Address, Bytes, Env, contract, contractimpl, panic_with_error, xdr::Error};

use crate::rwa::claim_issuer::{ClaimIssuer, ClaimIssuerError};

pub struct ClaimIssuerTrivial;

impl ClaimIssuer for ClaimIssuerTrivial {
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        scheme: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) {
        // should panic under some scenarios
        // todo ghost here.
        // TODO: Implement something here, not sure what would be appropriate.
    }
}

// this function needs to be implemented in a ClaimIssuer, even 
// though it is not part of the trait.
pub fn try_is_claim_valid(
    e: &Env,
    identity: Address,
    claim_topic: u32,
    scheme: u32,
    sig_data: Bytes,
    claim_data: Bytes,
) -> Result<Result<(), ClaimIssuerError>, Error> {
    // should return the same as above but instead of panicking should 
    // return an error.
    ClaimIssuerTrivial::is_claim_valid(e, identity, claim_topic, scheme, sig_data, claim_data);
    Ok(Ok(()))
}