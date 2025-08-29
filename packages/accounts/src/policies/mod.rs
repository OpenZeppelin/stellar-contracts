use soroban_sdk::{auth::Context, contractclient, Address, Env, Vec};

use crate::smart_account::storage::Signer;

mod test;
pub mod weighted_threshold;

// can be shared across multiple smart accounts or owned by only one
#[contractclient(name = "PolicyClient")]
pub trait Policy {
    // only verify that policy is enforceable, it should trigger no state changes,
    // because can be called multiple times for different context rules
    // configurations
    fn can_enforce(
        e: &Env,
        source: Address,
        context: Context,
        context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
    ) -> bool;

    // this serves as a hook and can trigger state changes and must be authorized by
    // the smart account (`source.require_auth()`)
    fn enforce(
        e: &Env,
        source: Address,
        context: Context,
        context_rule_signers: Vec<Signer>,
        authenticated_signers: Vec<Signer>,
    );
}
