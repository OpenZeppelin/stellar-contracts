use soroban_sdk::{Address, Bytes, Env, contract, contractimpl, panic_with_error, xdr::Error};

use crate::rwa::claim_issuer::{ClaimIssuer, ClaimIssuerError};

pub struct ClaimIssuerContract;

impl ClaimIssuer for ClaimIssuerContract {
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        scheme: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) {
        // TODO: Implement something here, not sure what would be appropriate.
    }
}

pub fn try_is_claim_valid(
    e: &Env,
    identity: Address,
    claim_topic: u32,
    scheme: u32,
    sig_data: Bytes,
    claim_data: Bytes,
) -> Result<Result<(), ClaimIssuerError>, Error> {
    ClaimIssuerContract::is_claim_valid(e, identity, claim_topic, scheme, sig_data, claim_data);
    Ok(Ok(()))
}