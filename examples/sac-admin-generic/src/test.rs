#![cfg(test)]
extern crate std;

use ed25519_dalek::{Signer, SigningKey, SECRET_KEY_LENGTH};
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

    let secret_key_chief: [u8; SECRET_KEY_LENGTH] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];
    let secret_key_operator: [u8; SECRET_KEY_LENGTH] = [
        57, 7, 177, 157, 29, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 234, 105,
        13, 50, 105, 25, 112, 59, 72, 3, 28, 174, 12, 34,
    ];
    // Generate signing keypairs.
    let chief = SigningKey::from_bytes(&secret_key_chief);
    let operator = SigningKey::from_bytes(&secret_key_operator);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());

    // Register the account contract, passing in the two signers (publickeys) to
    // // the constructor.
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
