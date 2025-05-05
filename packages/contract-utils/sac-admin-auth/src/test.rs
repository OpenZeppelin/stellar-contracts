#![cfg(test)]
extern crate std;

use ed25519_dalek::{Keypair, Signer};
use rand::thread_rng;
use soroban_sdk::{
    auth::{Context, ContractContext},
    testutils::{Address as _, BytesN as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec,
    xdr::{AccountId, PublicKey, ToXdr, Uint256},
    Address, BytesN, Env, IntoVal, Symbol,
};

use crate::admin::{Error, SacAdmin, Signature};

fn create_auth_context(e: &Env, contract: &Address, fn_name: Symbol, amount: i128) -> Context {
    Context::Contract(ContractContext {
        contract: contract.clone(),
        fn_name,
        args: ((), (), amount).into_val(e),
    })
}

#[test]
fn test_sac_transfer() {
    let e = Env::default();

    let issuer = Address::generate(&e);
    std::println!("issuer: {:?}", issuer.clone().to_xdr(&e));
    let owner = Address::generate(&e);
    std::println!("owner: {:?}", owner.clone().to_xdr(&e));
    let user1 = Address::generate(&e);
    //let user2 = Address::generate(&e);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());
    std::println!("sac: {:?}", sac_client.address);

    // Mint 1000 tokens to user1 from the SAC
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &issuer,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "mint",
            args: (&user1, 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    sac_client.mint(&user1, &1000);

    let token_client = TokenClient::new(&e, &sac.address());

    let balance1 = token_client.balance(&user1);
    assert_eq!(balance1, 1000);

    // Deploy the New Admin
    let new_admin = e.register(SacAdmin, (sac_client.address.clone(), owner.clone()));
    std::println!("new_admin: {:?}", new_admin);
    //let new_admin_client = SacAdminClient::new(&e, &new_admin);

    // Set the New Admin
    e.mock_auths(&[MockAuth {
        // issuer authorizes
        address: &issuer,
        invoke: &MockAuthInvoke {
            contract: &sac_client.address,
            fn_name: "set_admin",
            args: (&new_admin,).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    sac_client.set_admin(&new_admin);

    // Mint 1000 tokens to user2 from the New Admin
    //e.mock_auths(&[MockAuth {
    //// owner authorizes
    //address: &owner,
    //invoke: &MockAuthInvoke {
    //contract: &sac_client.address,
    //fn_name: "mint",
    //args: (&user2, 1000_i128).into_val(&e),
    //sub_invokes: &[],
    //},
    //}]);
    //
    // Can we actually test this?
    //sac_client.mint(&user2, &1000);
    //new_admin_client.mint(&user2, &1000);

    //let balance2 = token_client.balance(&user2);
    //assert_eq!(balance2, 1000);
}

#[test]
fn test_sac_admin_auth() {
    let e = Env::default();
    let issuer = Address::generate(&e);

    // Generate signing keypairs.
    let owner_keypair = Keypair::generate(&mut thread_rng());
    let signer_keypair = Keypair::generate(&mut thread_rng());

    std::println!("owner_public: {:?}", hex::encode(owner_keypair.public.as_bytes()));
    let owner_account =
        AccountId(PublicKey::PublicKeyTypeEd25519(Uint256::from(owner_keypair.public.to_bytes())));
    let owner = Address::from_str(&e, &std::string::ToString::to_string(&owner_account));
    std::println!("owner: {:?}", owner);

    // Deploy the Stellar Asset Contract
    let sac = e.register_stellar_asset_contract_v2(issuer.clone());
    let sac_client = StellarAssetClient::new(&e, &sac.address());
    std::println!("sac: {:?}", sac_client.address);

    // Register the account contract, passing in the two signers (public keys) to the constructor.
    let new_admin = e.register(
        SacAdmin,
        (sac.address(), BytesN::from_array(&e, owner_keypair.public.as_bytes())),
    );

    let payload = BytesN::random(&e);

    assert_eq!(
        e.try_invoke_contract_check_auth::<Error>(
            &new_admin,
            &payload,
            Signature {
                public_key: BytesN::from_array(&e, &owner_keypair.public.to_bytes()),
                signature: BytesN::from_array(
                    &e,
                    &owner_keypair.sign(payload.to_array().as_slice()).to_bytes()
                ),
            }
            .into_val(&e),
            &vec![&e, create_auth_context(&e, &sac_client.address, Symbol::new(&e, "mint"), 1000)],
        ),
        Ok(())
    );

    assert_eq!(
        e.try_invoke_contract_check_auth::<Error>(
            &new_admin,
            &payload,
            Signature {
                public_key: BytesN::from_array(&e, &signer_keypair.public.to_bytes()),
                signature: BytesN::from_array(
                    &e,
                    &signer_keypair.sign(payload.to_array().as_slice()).to_bytes()
                ),
            }
            .into_val(&e),
            &vec![&e, create_auth_context(&e, &sac_client.address, Symbol::new(&e, "mint"), 1000)],
        ),
        Err(Ok(Error::Unauthorized))
    );
}
