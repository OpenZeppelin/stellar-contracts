use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Address, BytesN, Env, IntoVal, Val, Vec,
};

use stellar_fungible::sac_admin_generic::{
    extract_context, get_sac_address, set_sac_address, SacFn,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SACAdminGenericError {
    Unauthorized = 1,
}

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
pub enum SacDataKey {
    Chief,
    Operator,
}

#[contract]
pub struct SacAdminExampleContract;

#[contractimpl]
impl SacAdminExampleContract {
    pub fn __constructor(e: Env, sac: Address, chief: BytesN<32>, operator: BytesN<32>) {
        set_sac_address(&e, &sac);
        e.storage().instance().set(&SacDataKey::Chief, &chief);
        e.storage().instance().set(&SacDataKey::Operator, &operator);
    }

    pub fn get_sac_address(e: &Env) -> Address {
        get_sac_address(e)
    }

    pub fn reassign_operator(e: &Env, operator: BytesN<32>) {
        e.current_contract_address().require_auth();
        e.storage().instance().set(&SacDataKey::Operator, &operator);
    }
}

#[contractimpl]
impl CustomAccountInterface for SacAdminExampleContract {
    type Signature = Signature;
    type Error = SACAdminGenericError;

    fn __check_auth(
        e: Env,
        payload: Hash<32>,
        signature: Self::Signature,
        auth_context: Vec<Context>,
    ) -> Result<(), SACAdminGenericError> {
        // authenticate
        e.crypto().ed25519_verify(
            &signature.public_key,
            &payload.clone().into(),
            &signature.signature,
        );
        let caller = signature.public_key.clone();

        // extract from context and check required permissionss for every function
        for context in auth_context.iter() {
            match extract_context(&e, &context) {
                SacFn::Mint(_amount) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator)?;
                    // ensure operator has minting limit
                }
                SacFn::Clawback(_amount) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator)?;
                    // ensure caller has clawback limit
                }
                SacFn::SetAuthorized(_) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator)?;
                }
                SacFn::SetAdmin => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Chief)?;
                }
                SacFn::Unknown => {
                    // ensure only chief can call other functions such as `reassign_operator()`
                    ensure_caller(&e, &caller, &SacDataKey::Chief)?
                }
            }
        }

        Ok(())
    }
}

fn ensure_caller<K: IntoVal<Env, Val>>(
    e: &Env,
    caller: &BytesN<32>,
    key: &K,
) -> Result<(), SACAdminGenericError> {
    let operator: BytesN<32> = e.storage().instance().get(key).expect("chief or operator not set");
    if *caller != operator {
        return Err(SACAdminGenericError::Unauthorized);
    }
    Ok(())
}
