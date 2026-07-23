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
    // Mirror the real SAC `mint(to, amount)` argument layout: `ContractContext`
    // carries only the invocation arguments, so `to` is at index 0 and `amount`
    // at index 1 (no `Env` slot).
    Context::Contract(ContractContext {
        contract: contract.clone(),
        fn_name,
        args: (Address::generate(e), amount).into_val(e),
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

    // Register the account contract, passing in the two signers (public keys) to
    // the constructor.
    let new_admin = e.register(
        SacAdminExampleContract,
        (
            sac.address(),
            BytesN::from_array(&e, chief.verifying_key().as_bytes()),
            BytesN::from_array(&e, operator.verifying_key().as_bytes()),
            1_000_000_000i128,
            0i128,
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

/// End-to-end test that drives a real Stellar Asset Contract through the
/// generic admin contract's `__check_auth`, calling `sac_client.mint()` and
/// `sac_client.clawback()`.
///
/// `mock_all_auths` bypasses `__check_auth`, so the real authorization path can
/// only be exercised by building and signing a `SorobanAuthorizationEntry` the
/// same way the Soroban host does and installing it with `set_auths`. This is
/// what catches the argument-index bug that `try_invoke_contract_check_auth`
/// with a hand-built context cannot.
#[test]
fn test_sac_generic_e2e_mint_and_clawback() {
    use soroban_sdk::{
        testutils::IssuerFlags,
        xdr::{
            Hash, HashIdPreimage, HashIdPreimageSorobanAuthorization, InvokeContractArgs, Limits,
            ScAddress, ScSymbol, ScVal, SorobanAddressCredentials, SorobanAuthorizationEntry,
            SorobanAuthorizedFunction, SorobanAuthorizedInvocation, SorobanCredentials, VecM,
            WriteXdr,
        },
        Bytes, TryFromVal, Val,
    };

    let e = Env::default();

    // Operator ed25519 key (same seed as the CLI-signed testnet run).
    let operator = SigningKey::from_bytes(&[
        57, 7, 177, 157, 29, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 234, 105,
        13, 50, 105, 25, 112, 59, 72, 3, 28, 174, 12, 34,
    ]);
    let operator_pk = operator.verifying_key().to_bytes();

    // Deploy a real SAC. `register_stellar_asset_contract_v2` creates a separate
    // issuer account internally; enabling clawback on it makes minted balances
    // clawbackable.
    let initial_admin = Address::generate(&e);
    let sac = e.register_stellar_asset_contract_v2(initial_admin);
    sac.issuer().set_flag(IssuerFlags::ClawbackEnabledFlag);
    let sac_client = StellarAssetClient::new(&e, &sac.address());

    // Deploy the generic SAC-admin contract (operator doubles as chief here).
    let admin = e.register(
        SacAdminExampleContract,
        (
            sac.address(),
            BytesN::from_array(&e, &operator_pk),
            BytesN::from_array(&e, &operator_pk),
            1_000_000_000i128,
            0i128,
        ),
    );

    // Hand the SAC admin role to the generic admin contract (authorized by the
    // initial admin via mocked auth — this is setup, not the code under test).
    e.mock_all_auths();
    sac_client.set_admin(&admin);

    // `Val` -> `ScVal`, matching how the host serializes the recorded call args.
    let sc = |v: Val| ScVal::try_from_val(&e, &v).unwrap();

    // Build a `SorobanAuthorizationEntry` signed by the operator, exactly as the
    // host presents the SAC invocation to `__check_auth`. `set_auths` also
    // disables the `mock_all_auths` enabled above.
    let build_auth = |fn_name: &str, args: std::vec::Vec<ScVal>, nonce: i64| {
        let exp = e.ledger().sequence() + 1000;
        let invocation = SorobanAuthorizedInvocation {
            function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                contract_address: ScAddress::from(sac.address()),
                function_name: ScSymbol(fn_name.try_into().unwrap()),
                args: args.try_into().unwrap(),
            }),
            sub_invocations: VecM::default(),
        };
        let preimage = HashIdPreimage::SorobanAuthorization(HashIdPreimageSorobanAuthorization {
            network_id: Hash(e.ledger().network_id().to_array()),
            nonce,
            signature_expiration_ledger: exp,
            invocation: invocation.clone(),
        });
        let payload = preimage.to_xdr(Limits::none()).unwrap();
        let payload_hash = e.crypto().sha256(&Bytes::from_slice(&e, &payload)).to_array();
        let signature = Signature {
            public_key: BytesN::from_array(&e, &operator_pk),
            signature: BytesN::from_array(&e, &operator.sign(&payload_hash).to_bytes()),
        };
        let signature: Val = signature.into_val(&e);
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: ScAddress::from(&admin),
                nonce,
                signature_expiration_ledger: exp,
                signature: sc(signature),
            }),
            root_invocation: invocation,
        }
    };

    let recipient = Address::generate(&e);
    let recipient_arg = sc(recipient.clone().into_val(&e));

    // mint(recipient, 1000) through the real SAC, authorized by `__check_auth`.
    e.set_auths(&[build_auth(
        "mint",
        std::vec![recipient_arg.clone(), sc(1000i128.into_val(&e))],
        1,
    )]);
    sac_client.mint(&recipient, &1000);
    assert_eq!(sac_client.balance(&recipient), 1000);

    // clawback(recipient, 400) through the real SAC, authorized by `__check_auth`.
    e.set_auths(&[build_auth("clawback", std::vec![recipient_arg, sc(400i128.into_val(&e))], 2)]);
    sac_client.clawback(&recipient, &400);
    assert_eq!(sac_client.balance(&recipient), 600);
}
