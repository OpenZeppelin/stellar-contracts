pub mod ed25519;
pub mod utils;
use soroban_sdk::{contractclient, Bytes, Env};

#[contractclient(name = "VerifierClient")]
pub trait Verifier {
    fn verify(e: &Env, hash: Bytes, sig_data: Bytes) -> bool;
}
