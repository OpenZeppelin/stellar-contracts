use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Address, BytesN, Env, IntoVal, Val, Vec,
};
use stellar_fungible::sac_admin_generic::{
    extract_sac_contract_context, get_sac_address, set_sac_address, SacFn,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SACAdminGenericError {
    Unauthorized = 1,
    InvalidContext = 2,
    MintingLimitExceeded = 3,
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
    Operator(BytesN<32>),     // -> true/false
    MintingLimit(BytesN<32>), // -> (max_limit, curr)
}

#[contract]
pub struct SacAdminExampleContract;

#[contractimpl]
impl SacAdminExampleContract {
    pub fn __constructor(e: Env, sac: Address, chief: BytesN<32>, operator: BytesN<32>) {
        set_sac_address(&e, &sac);
        e.storage().instance().set(&SacDataKey::Chief, &chief);
        e.storage().instance().set(&SacDataKey::Operator(operator.clone()), &true);
        e.storage().instance().set(&SacDataKey::MintingLimit(operator), &(1_000_000_000, 0i128));
    }

    pub fn get_sac_address(e: &Env) -> Address {
        get_sac_address(e)
    }

    pub fn assign_operator(e: &Env, operator: BytesN<32>) {
        e.current_contract_address().require_auth();
        e.storage().instance().set(&SacDataKey::Operator(operator), &true);
    }

    pub fn remove_operator(e: &Env, operator: BytesN<32>) {
        e.current_contract_address().require_auth();
        e.storage().instance().remove(&SacDataKey::Operator(operator));
    }

    // set or reset
    pub fn set_minting_limit(e: &Env, operator: BytesN<32>, limit: i128) {
        e.current_contract_address().require_auth();
        e.storage().instance().set(&SacDataKey::MintingLimit(operator), &(limit, 0i128));
    }

    pub fn update_minting_limit(e: &Env, operator: BytesN<32>, new_limit: i128) {
        e.current_contract_address().require_auth();
        let key = SacDataKey::MintingLimit(operator);
        let (_, curr): (i128, i128) = e.storage().instance().get(&key).expect("limit not set");
        e.storage().instance().set(&key, &(new_limit, curr));
    }
}

#[contractimpl]
impl CustomAccountInterface for SacAdminExampleContract {
    type Error = SACAdminGenericError;
    type Signature = Signature;

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
        for ctx in auth_context.iter() {
            let context = match ctx {
                Context::Contract(c) => c,
                _ => return Err(SACAdminGenericError::InvalidContext),
            };

            match extract_sac_contract_context(&e, &context) {
                SacFn::Mint(amount) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator(caller.clone()))?;
                    // ensure operator has minting limit
                    ensure_minting_limit(&e, &caller, amount)?;
                }
                SacFn::Clawback(_amount) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator(caller.clone()))?;
                }
                SacFn::SetAuthorized(_) => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Operator(caller.clone()))?;
                }
                SacFn::SetAdmin => {
                    // ensure caller has required permissions
                    ensure_caller(&e, &caller, &SacDataKey::Chief)?;
                }
                SacFn::Unknown => {
                    // ensure only chief can call other functions such as `assign_operator()`,
                    // `remove_operator()` or `set_minting_limit()`
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

fn ensure_minting_limit(
    e: &Env,
    caller: &BytesN<32>,
    amount: i128,
) -> Result<(), SACAdminGenericError> {
    let key = SacDataKey::MintingLimit(caller.clone());

    let (max, curr): (i128, i128) = e.storage().instance().get(&key).expect("limit not set");
    let new_limit: i128 = curr.checked_add(amount).expect("overflow");
    if new_limit > max {
        return Err(SACAdminGenericError::MintingLimitExceeded);
    }

    // update
    e.storage().instance().set(&key, &(max, new_limit));
    Ok(())
}
