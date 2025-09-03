// TODO: re-enable the below when soroban-sdk is updated to support having the
// same function names across different contracts

// use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address,
// Env, String}; use stellar_macros::default_impl;
// use stellar_tokens::fungible::{
//     vault::{FungibleVault, Vault},
//     Base, FungibleToken,
// };

// #[contract]
// pub struct AssetContract;

// #[contractimpl]
// impl AssetContract {
//     pub fn __constructor(e: &Env) {
//         Base::set_metadata(e, 18, String::from_str(e, "Asset Token"),
// String::from_str(e, "ASSET"));     }

//     pub fn mint(e: &Env, to: Address, amount: i128) {
//         Base::mint(e, &to, amount);
//     }
// }

// #[default_impl]
// #[contractimpl]
// impl FungibleToken for AssetContract {
//     type ContractType = Base;
// }

// #[contract]
// pub struct VaultContract;

// #[contractimpl]
// impl VaultContract {
//     pub fn __constructor(e: &Env, asset: Address) {
//         Base::set_metadata(e, 18, String::from_str(e, "Vault Token"),
// String::from_str(e, "vTKN"));         Vault::set_asset(e, asset);
//     }
// }

// #[default_impl]
// #[contractimpl]
// impl FungibleToken for VaultContract {
//     type ContractType = Vault;
// }

// #[default_impl]
// #[contractimpl]
// impl FungibleVault for VaultContract {}

// fn create_asset_client<'a>(e: &Env) -> AssetContractClient<'a> {
//     let address = e.register(AssetContract, ());
//     let client = AssetContractClient::new(e, &address);
//     client
// }

// fn create_vault_client<'a>(e: &Env, asset_address: &Address) ->
// VaultContractClient<'a> {     let address = e.register(VaultContract,
// (asset_address,));     let client = VaultContractClient::new(e, &address);
//     client
// }

// #[test]
// fn default_impl_vault_query_asset() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);

//     // The asset address should match what was set during construction
//     assert_eq!(vault_client.query_asset(), asset_client.address);
// }

// #[test]
// fn default_impl_vault_total_assets() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Initially no assets
//     assert_eq!(vault_client.total_assets(), 0);

//     // Deposit some assets
//     vault_client.deposit(&1000, &user, &user);
//     assert_eq!(vault_client.total_assets(), 1000);
// }

// #[test]
// fn default_impl_vault_convert_to_shares() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Initial conversion when vault is empty (1:1 ratio)
//     assert_eq!(vault_client.convert_to_shares(&1000), 1000);

//     // After depositing assets and minting shares
//     vault_client.deposit(&1000, &user, &user);
//     vault_client.mint(&1000, &user, &user);

//     // Should still be 1:1 ratio
//     assert_eq!(vault_client.convert_to_shares(&500), 500);
// }

// #[test]
// fn default_impl_vault_convert_to_assets() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Initial conversion when vault is empty (1:1 ratio)
//     assert_eq!(vault_client.convert_to_assets(&1000), 1000);

//     // After depositing assets and minting shares
//     vault_client.deposit(&1000, &user, &user);
//     vault_client.mint(&1000, &user, &user);

//     // Should still be 1:1 ratio
//     assert_eq!(vault_client.convert_to_assets(&500), 500);
// }

// #[test]
// fn default_impl_vault_max_deposit() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     // Should return maximum i128 value
//     assert_eq!(vault_client.max_deposit(&user), i128::MAX);
// }

// #[test]
// fn default_impl_vault_preview_deposit() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Preview should match convert_to_shares
//     let assets = 1000;
//     let expected_shares = vault_client.convert_to_shares(&assets);
//     assert_eq!(vault_client.preview_deposit(&assets), expected_shares);

//     // After some activity
//     vault_client.deposit(&500, &user, &user);
//     vault_client.mint(&500, &user, &user);

//     let new_expected_shares = vault_client.convert_to_shares(&assets);
//     assert_eq!(vault_client.preview_deposit(&assets), new_expected_shares);
// }

// #[test]
// fn default_impl_vault_max_mint() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     // Should return maximum i128 value
//     assert_eq!(vault_client.max_mint(&user), i128::MAX);
// }

// #[test]
// fn default_impl_vault_preview_mint() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Preview mint should show required assets for shares
//     let shares = 1000;
//     let required_assets = vault_client.preview_mint(&shares);

//     // Initially should be 1:1 ratio
//     assert_eq!(required_assets, shares);

//     // After some activity
//     vault_client.deposit(&500, &user, &user);
//     vault_client.mint(&500, &user, &user);

//     let new_required_assets = vault_client.preview_mint(&shares);
//     assert_eq!(new_required_assets, shares); // Still 1:1 in this simple case
// }

// #[test]
// fn default_impl_vault_max_withdraw() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Initially user has no shares, so can't withdraw anything
//     assert_eq!(vault_client.max_withdraw(&user), 0);

//     // After minting shares
//     vault_client.deposit(&1000, &user, &user);
//     vault_client.mint(&500, &user, &user);

//     // Should be able to withdraw equivalent assets
//     assert_eq!(vault_client.max_withdraw(&user), 500);
// }

// #[test]
// fn default_impl_vault_preview_withdraw() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);

//     e.mock_all_auths();

//     // Preview withdraw should show required shares for assets
//     let assets = 500;
//     let required_shares = vault_client.preview_withdraw(&assets);

//     // Initially should be 1:1 ratio
//     assert_eq!(required_shares, assets);
// }

// #[test]
// fn default_impl_vault_max_redeem() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Initially user has no shares
//     assert_eq!(vault_client.max_redeem(&user), 0);

//     // After minting shares
//     vault_client.mint(&500, &user, &user);

//     // Should be able to redeem all shares
//     assert_eq!(vault_client.max_redeem(&user), 500);
// }

// #[test]
// fn default_impl_vault_preview_redeem() {
//     let e = Env::default();
//     let asset_client = create_asset_client(&e);
//     let vault_client = create_vault_client(&e, &asset_client.address);
//     let user = Address::generate(&e);

//     e.mock_all_auths();

//     // Preview redeem should match convert_to_assets
//     let shares = 500;
//     let expected_assets = vault_client.convert_to_assets(&shares);
//     assert_eq!(vault_client.preview_redeem(&shares), expected_assets);

//     // After some activity
//     vault_client.deposit(&1000, &user, &user);
//     vault_client.mint(&500, &user, &user);

//     let new_expected_assets = vault_client.convert_to_assets(&shares);
//     assert_eq!(vault_client.preview_redeem(&shares), new_expected_assets);
// }
