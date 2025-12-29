use cvlr::{clog, nondet::{nondet, Nondet}};
use cvlr_soroban::nondet_address;
use soroban_sdk::panic_with_error;
use soroban_sdk::{contract, contractimpl, Address, Env};

use crate::rwa::identity_verifier::{storage, IdentityVerifier};
use crate::rwa::RWAError;

pub struct IdentityVerifierTrivial; 

use crate::rwa::specs::ghosts::GhostMap;
pub static mut VERIFY_IDENTITY_RESULT_MAP: GhostMap<Address, bool> = GhostMap::UnInit;

impl IdentityVerifier for IdentityVerifierTrivial {
    fn verify_identity(e: &Env, account: &Address) {
        clog!(cvlr_soroban::Addr(account));
        unsafe {
            let bool: bool = VERIFY_IDENTITY_RESULT_MAP.get(account);
            clog!(bool);
            if !bool {
                panic_with_error!(e, RWAError::IdentityVerificationFailed);
            }
        }
    }

    fn recovery_target(e: &Env, old_account: &Address) -> Option<Address> {
        let bool: bool = nondet();
        if !bool {
            return None;
        }
        Some(nondet_address())
    }

    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address) {
        // do nothing
    }

    fn claim_topics_and_issuers(e: &Env) -> Address {
        nondet_address()
    }

}

impl IdentityVerifierTrivial {
    pub fn verify_identity_non_panicking(e: &Env, account: &Address) -> bool {
        clog!(cvlr_soroban::Addr(account));
        unsafe {
            let bool: bool = VERIFY_IDENTITY_RESULT_MAP.get(account);
            clog!(bool);
            bool
        }
    }
}