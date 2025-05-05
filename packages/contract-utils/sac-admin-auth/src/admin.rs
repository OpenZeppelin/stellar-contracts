use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Address, BytesN, Env, Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    UnknownSigner = 1,
    InvalidContext = 2,
    InvalidFn = 3,
    Unauthorized = 4,
}

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
pub enum SacDataKey {
    SAC,
    Owner,
}

#[contract]
pub struct SacAdmin;

#[contractimpl]
impl SacAdmin {
    pub fn __constructor(e: Env, sac: Address, owner: BytesN<32>) {
        e.storage().instance().set(&SacDataKey::SAC, &sac);

        // for the sake of the example, using owner instead of roles
        e.storage().instance().set(&SacDataKey::Owner, &owner);
    }
}

#[contractimpl]
impl CustomAccountInterface for SacAdmin {
    type Signature = Signature;
    type Error = Error;

    fn __check_auth(
        e: Env,
        payload: Hash<32>,
        signature: Self::Signature,
        auth_context: Vec<Context>,
    ) -> Result<(), Error> {
        // authenticate
        e.crypto().ed25519_verify(
            &signature.public_key,
            &payload.clone().into(),
            &signature.signature,
        );

        // extract from context and check roles for every function
        for context in auth_context.iter() {
            let contract_context = match context {
                Context::Contract(c) => c,
                _ => return Err(Error::InvalidContext),
            };

            let sac_address: Address = e.storage().instance().get(&SacDataKey::SAC).unwrap();
            if contract_context.contract != sac_address {
                return Err(Error::InvalidContext);
            }

            let caller = signature.public_key.clone();

            if contract_context.fn_name == Symbol::new(&e, "mint") {
                // for the sake of the example, checking against owner only
                let owner: BytesN<32> = e.storage().instance().get(&SacDataKey::Owner).unwrap();
                if caller != owner {
                    return Err(Error::Unauthorized);
                }
                //ensure_has_role("minter", caller)

                //let amount_bytes = contract_context.args.get(1).unwrap().to_xdr(&e);
                //let amount = i128::from_xdr(&e, &amount_bytes).unwrap();
                //ensure_has_limit(amount, pub_key)
            } else if contract_context.fn_name == Symbol::new(&e, "clawback") {
                // ensure_has_role("clawback", caller)
            } else {
                return Err(Error::InvalidFn);
            }
        }

        Ok(())
    }
}
