pub mod ed25519;
mod test;
pub mod utils;
pub mod webauthn;
use soroban_sdk::{contractclient, Bytes, Env, FromVal, Val};

pub trait Verifier {
    type SigData: FromVal<Env, Val>;

    fn verify(e: &Env, hash: Bytes, key_data: Bytes, sig_data: Bytes) -> bool;
}

// We need to declare a `VerifierClientInterface` here, instead of using the
// public trait above, because traits with associated types are not supported
// by the `#[contractclient]` macro. While this may appear redundant, it's a
// necessary workaround: we declare an identical internal trait with the macro
// to generate the required client implementation. Users should only interact
// with the public `Verifier` trait above for their implementations.
#[allow(unused)]
#[contractclient(name = "VerifierClient")]
trait VerifierClientInterface {
    fn verify(e: &Env, hash: Bytes, key_data: Bytes, sig_data: Bytes) -> bool;
}
