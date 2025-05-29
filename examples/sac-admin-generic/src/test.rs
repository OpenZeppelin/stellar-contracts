#![cfg(test)]
extern crate std;

use ed25519_dalek::{Keypair, Signer};
use rand::thread_rng;
use soroban_sdk::{
    auth::{Context, ContractContext},
    testutils::{Address as _, BytesN as _},
    token::StellarAssetClient,
    vec,
    //xdr::{AccountId, PublicKey, Uint256},
    Address,
    BytesN,
    Env,
    IntoVal,
    Symbol,
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

    // Generate signing keypairs.
    let chief_keypair = Keypair::generate(&mut thread_rng());
    let operator_keypair = Keypair::generate(&mut thread_rng());

    //std::println!("owner_public: {:?}", hex::encode(chief_keypair.public.as_bytes()));
    //let owner_account =
    //AccountId(PublicKey::PublicKeyTypeEd25519(Uint256::from(chief_keypair.public.to_bytes())));
    //let owner = Address::from_str(&e, &std::string::ToString::to_string(&owner_account));
    //std::println!("owner: {:?}", owner);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());
    //std::println!("sac: {:?}", sac_client.address);

    // Register the account contract, passing in the two signers (public keys) to the constructor.
    let new_admin = e.register(
        SacAdminExampleContract,
        (
            sac.address(),
            BytesN::from_array(&e, chief_keypair.public.as_bytes()),
            BytesN::from_array(&e, operator_keypair.public.as_bytes()),
        ),
    );

    let payload = BytesN::random(&e);

    assert_eq!(
        e.try_invoke_contract_check_auth::<SACAdminGenericError>(
            &new_admin,
            &payload,
            Signature {
                public_key: BytesN::from_array(&e, &operator_keypair.public.to_bytes()),
                signature: BytesN::from_array(
                    &e,
                    &operator_keypair.sign(payload.to_array().as_slice()).to_bytes()
                ),
            }
            .into_val(&e),
            &vec![&e, create_auth_context(&e, &sac_client.address, Symbol::new(&e, "mint"), 1000)],
        ),
        Ok(())
    );
}
