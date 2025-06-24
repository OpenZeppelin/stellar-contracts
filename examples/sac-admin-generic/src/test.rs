#![cfg(test)]
extern crate std;

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    auth::{Context, ContractContext},
    testutils::{Address as _, BytesN as _},
    token::StellarAssetClient,
    vec, Address, BytesN, Env, IntoVal, Symbol,
};

use crate::contract::{SACAdminGenericError, SacAdminExampleContract, Signature};

fn create_auth_context(e: &Env, contract: &Address, fn_name: Symbol, amount: i128) -> Context {
    Context::Contract(ContractContext {
        contract: contract.clone(),
        fn_name,
        args: ((), (), amount).into_val(e),
    })
}

#[test]
fn test_sac_generic() {
    let e = Env::default();
    let issuer = Address::generate(&e);

    let mut csprng = OsRng;
    // Generate signing keypairs.
    let chief = SigningKey::generate(&mut csprng);
    let operator = SigningKey::generate(&mut csprng);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());

    // Register the account contract, passing in the two signers (public keys) to
    // the constructor.
    let new_admin = e.register(
        SacAdminExampleContract,
        (
            sac.address(),
            BytesN::from_array(&e, chief.verifying_key().as_bytes()),
            BytesN::from_array(&e, operator.verifying_key().as_bytes()),
        ),
    );

    let payload = BytesN::random(&e);

    assert_eq!(
        e.try_invoke_contract_check_auth::<SACAdminGenericError>(
            &new_admin,
            &payload,
            Signature {
                public_key: BytesN::from_array(&e, &operator.verifying_key().to_bytes()),
                signature: BytesN::from_array(
                    &e,
                    &operator.sign(payload.to_array().as_slice()).to_bytes()
                ),
            }
            .into_val(&e),
            &vec![&e, create_auth_context(&e, &sac_client.address, Symbol::new(&e, "mint"), 1000)],
        ),
        Ok(())
    );
}
