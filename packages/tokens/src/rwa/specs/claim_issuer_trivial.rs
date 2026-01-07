use core::hash;

use soroban_sdk::{Address, Bytes, Env, contract, contractimpl, panic_with_error, xdr::Error};

use crate::rwa::claim_issuer::{ClaimIssuer, ClaimIssuerError};

pub struct ClaimIssuerTrivial;

use crate::rwa::specs::ghosts::GhostMap;
pub static mut IS_CLAIM_VALID_RESULT_MAP: GhostMap<(Address, u32, u32, Bytes, Bytes), bool> = GhostMap::UnInit;

use soroban_sdk::contracterror;
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ClaimIssuerTrivialError {
    ClaimNotValid = 360,
}

impl ClaimIssuer for ClaimIssuerTrivial {
    fn is_claim_valid(
        e: &Env,
        identity: Address,
        claim_topic: u32,
        scheme: u32,
        sig_data: Bytes,
        claim_data: Bytes,
    ) {
        unsafe {
            let bool: bool = IS_CLAIM_VALID_RESULT_MAP.get(&(identity, claim_topic, scheme, sig_data, claim_data));
            if !bool {
                panic_with_error!(e, ClaimIssuerTrivialError::ClaimNotValid);
            }
        }
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
    unsafe {
        let bool: bool = IS_CLAIM_VALID_RESULT_MAP.get(&(identity, claim_topic, scheme, sig_data, claim_data));
        if !bool {
            return Ok(Err(ClaimIssuerError::NotAllowed));
        }
        Ok(Ok(()))
    }
}