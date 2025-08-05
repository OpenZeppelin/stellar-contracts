#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, IntoVal, Map, Symbol, Val, Vec,
};
use stellar_diamond_proxy_core::{storage::Storage, Error};

#[contracttype]
#[derive(Clone)]
pub struct TokenConfig {
    pub name: Symbol,
    pub symbol: Symbol,
    pub decimals: u32,
    pub max_supply: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct SettlementConfig {
    pub settlement_token: BytesN<32>,
    pub settlement_period: u64,
    pub min_amount: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct DiamondConfig {
    pub owner: Address,
    pub facets: Map<Symbol, BytesN<32>>,
}

#[contracttype]
#[derive(Clone)]
pub struct DiamondInit {
    pub owner: BytesN<32>,
    pub facets: Map<Symbol, BytesN<32>>,
}

#[contracttype]
#[derive(Clone)]
pub struct FacetWasmInfo {
    pub wasm: BytesN<32>, // WASM hash
    pub initialization_args: Option<Vec<Val>>,
}

#[contracttype]
#[derive(Clone)]
pub struct DeploymentPlan {
    pub dependencies: Map<Symbol, Vec<Symbol>>,
    pub order: Vec<Symbol>,
}

#[contracttype]
#[derive(Clone)]
pub struct DiamondFactoryState {
    pub owner: Address,
    pub diamonds: Map<BytesN<32>, DiamondConfig>,
}

#[contract]
pub struct DiamondFactory;

#[contractimpl]
impl DiamondFactory {
    pub fn init(env: Env, owner: Address) {
        let storage = Storage::new(env.clone());
        storage.require_uninitialized();
        storage.set_owner(&owner);
        storage.set_initialized();
    }

    pub fn deploy_diamond(
        env: Env,
        owner: Address,
        wasm_hash: BytesN<32>,
        salt: BytesN<32>,
    ) -> Result<Address, Error> {
        let storage = Storage::new(env.clone());
        if !storage.is_initialized() {
            return Err(Error::DiamondProxyNotInitialized);
        }

        let stored_owner = storage.get_owner().ok_or(Error::NotFound)?;
        if owner != stored_owner {
            return Err(Error::Unauthorized);
        }

        // Explicitly require authorization from the owner
        owner.require_auth();

        let diamond_addr = env
            .deployer()
            .with_address(owner.clone(), salt.clone())
            .deploy_v2(wasm_hash, ());

        let salt_prime = env.crypto().keccak256(&soroban_sdk::Bytes::from(&salt));
        let salt_for_diamond: BytesN<32> = BytesN::from(salt_prime);

        // Call init on the Diamond
        env.invoke_contract::<Result<(), Error>>(
            &diamond_addr,
            &Symbol::new(&env, "init"),
            Vec::from_array(
                &env,
                [
                    owner.clone().into_val(&env),
                    salt_for_diamond.into_val(&env),
                ],
            ),
        )
        .map_err(|_| Error::DiamondInitFailed)?;

        // Return the wasm hash as the diamond address
        Ok(diamond_addr)
    }
}
