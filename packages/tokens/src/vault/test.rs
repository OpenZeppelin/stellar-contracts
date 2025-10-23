extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};

use crate::{
    fungible::Base,
    vault::{Vault, MAX_DECIMALS_OFFSET},
};

// Simple mock contract for vault testing
#[contract]
struct MockVaultContract;

// Mock Asset Contract - Implements balance, transfer, transfer_from, approve,
// and decimals
#[contract]
struct MockAssetContract;

#[contractimpl]
impl MockAssetContract {
    pub fn balance(e: &Env, id: Address) -> i128 {
        e.storage().temporary().get(&id).unwrap_or(0)
    }

    pub fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        let from_balance = Self::balance(e, from.clone());
        let to_balance = Self::balance(e, to.clone());

        e.storage().temporary().set(&from, &(from_balance - amount));
        e.storage().temporary().set(&to, &(to_balance + amount));
    }

    pub fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        // Get allowance
        let allowance_key = (from.clone(), spender.clone());
        let allowance: i128 = e.storage().temporary().get(&allowance_key).unwrap_or(0);

        // Check if allowance is sufficient
        if allowance < amount {
            panic!("Insufficient allowance");
        }

        // Update allowance
        e.storage().temporary().set(&allowance_key, &(allowance - amount));

        // Transfer tokens
        let from_balance = Self::balance(e, from.clone());
        let to_balance = Self::balance(e, to.clone());
        e.storage().temporary().set(&from, &(from_balance - amount));
        e.storage().temporary().set(&to, &(to_balance + amount));
    }

    pub fn approve(
        e: &Env,
        from: Address,
        spender: Address,
        amount: i128,
        _expiration_ledger: u32,
    ) {
        let allowance_key = (from, spender);
        e.storage().temporary().set(&allowance_key, &amount);
    }

    pub fn decimals(_e: &Env) -> u32 {
        18
    }

    // Helper function to mint tokens for testing
    pub fn mint(e: &Env, to: Address, amount: i128) {
        let balance = Self::balance(e, to.clone());
        e.storage().temporary().set(&to, &(balance + amount));
    }
}

fn create_vault_contract(e: &Env, asset_address: &Address, decimals_offset: u32) -> Address {
    let vault_address = e.register(MockVaultContract, ());
    e.as_contract(&vault_address, || {
        Vault::set_asset(e, asset_address.clone());
        Vault::set_decimals_offset(e, decimals_offset);
    });
    vault_address
}

fn create_asset_contract(e: &Env, initial_supply: i128, admin: &Address) -> Address {
    let asset_address = e.register(MockAssetContract, ());
    let asset_client = MockAssetContractClient::new(e, &asset_address);
    asset_client.mint(admin, &initial_supply);
    asset_address
}

#[test]
fn vault_asset_address() {
    let e = Env::default();
    let asset_address = Address::generate(&e);
    let vault_address = create_vault_contract(&e, &asset_address, 6);

    e.as_contract(&vault_address, || {
        let queried_asset = Vault::query_asset(&e);
        assert_eq!(queried_asset, asset_address);
    });
}

#[test]
fn vault_decimals_offset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create asset contract (18 decimals)
    let asset_address = create_asset_contract(&e, initial_supply, &admin);

    // Create vault contract with decimals offset
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        // Vault decimals should be asset decimals + offset
        assert_eq!(Vault::decimals(&e), 18 + decimals_offset);
        assert_eq!(Vault::get_decimals_offset(&e), decimals_offset);
    });
}

#[test]
fn vault_total_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        // Initially, vault should have 0 assets
        assert_eq!(Vault::total_assets(&e), 0);
    });

    // Transfer some assets to vault
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    let transfer_amount = 100_000_000_000_000_000i128;
    asset_client.transfer(&admin, &vault_address, &transfer_amount);

    e.as_contract(&vault_address, || {
        // Now vault should have the transferred assets
        assert_eq!(Vault::total_assets(&e), transfer_amount);
    });
}

#[test]
fn conversion_functions_empty_vault() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        let assets = 1_000_000_000_000_000_000i128; // 1 token
        let expected_shares = assets * 10i128.pow(decimals_offset);

        // Test conversions with empty vault
        assert_eq!(Vault::convert_to_shares(&e, assets), expected_shares);
        assert_eq!(Vault::convert_to_assets(&e, expected_shares), assets);

        // Test preview functions
        assert_eq!(Vault::preview_deposit(&e, assets), expected_shares);
        assert_eq!(Vault::preview_mint(&e, expected_shares), assets);
        assert_eq!(Vault::preview_withdraw(&e, assets), expected_shares);
        assert_eq!(Vault::preview_redeem(&e, expected_shares), assets);
    });
}

#[test]
fn max_functions() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        // Test max functions with empty vault
        assert_eq!(Vault::max_deposit(&e, user.clone()), i128::MAX);
        assert_eq!(Vault::max_mint(&e, user.clone()), i128::MAX);
        assert_eq!(Vault::max_withdraw(&e, user.clone()), 0); // No shares yet
        assert_eq!(Vault::max_redeem(&e, user.clone()), 0); // No shares yet
    });
}

#[test]
fn deposit_functionality() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        // Test deposit functionality
        let shares_minted =
            Vault::deposit(&e, deposit_amount, user.clone(), admin.clone(), admin.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_minted);
        assert_eq!(Base::total_supply(&e), shares_minted);
        assert_eq!(Vault::total_assets(&e), deposit_amount);

        // For first deposit, shares should equal assets with offset
        assert_eq!(shares_minted, deposit_amount * 10i128.pow(decimals_offset));
    });
}

#[test]
fn mint_functionality() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let shares_to_mint = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        let required_assets = Vault::preview_mint(&e, shares_to_mint);

        let assets_deposited =
            Vault::mint(&e, shares_to_mint, user.clone(), user.clone(), user.clone());

        assert_eq!(Base::balance(&e, &user), shares_to_mint);
        assert_eq!(Base::total_supply(&e), shares_to_mint);
        assert_eq!(assets_deposited, required_assets);
    });
}

#[test]
fn withdraw_functionality() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;
    let withdraw_amount = 50_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        let shares_minted =
            Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone());

        // Withdraw assets
        let shares_burned =
            Vault::withdraw(&e, withdraw_amount, user.clone(), user.clone(), user.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_minted - shares_burned);
        assert_eq!(Vault::total_assets(&e), deposit_amount - withdraw_amount);
    });
}

#[test]
fn redeem_functionality() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        let shares_minted =
            Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone());

        // Redeem half the shares
        let shares_to_redeem = shares_minted / 2;
        let assets_received =
            Vault::redeem(&e, shares_to_redeem, user.clone(), user.clone(), user.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_minted - shares_to_redeem);
        assert_eq!(Base::total_supply(&e), shares_minted - shares_to_redeem);

        // Should receive approximately half the original deposit
        let expected_assets = deposit_amount / 2;
        assert!(assets_received >= expected_assets - 1 && assets_received <= expected_assets + 1);
    });
}

#[test]
fn conversion_with_existing_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    // Setup: deposit some assets first
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    asset_client.transfer(&admin, &vault_address, &deposit_amount);

    e.as_contract(&vault_address, || {
        Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone());

        // Test conversions with vault having assets
        let new_assets = 50_000_000_000_000_000i128;
        let shares = Vault::convert_to_shares(&e, new_assets);
        let converted_back = Vault::convert_to_assets(&e, shares);

        // Should be approximately equal (allowing for rounding)
        assert!(converted_back >= new_assets - 1 || converted_back <= new_assets + 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #407)")]
fn withdraw_exceeds_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    // Setup: deposit assets first
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    asset_client.transfer(&admin, &vault_address, &deposit_amount);

    e.as_contract(&vault_address, || {
        Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone());

        // Try to withdraw more than max
        let max_withdraw = Vault::max_withdraw(&e, user.clone());
        Vault::withdraw(&e, max_withdraw + 1, user.clone(), user.clone(), user.clone());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #408)")]
fn redeem_exceeds_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.mock_all_auths();

    // Setup: deposit assets first
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    asset_client.transfer(&admin, &vault_address, &deposit_amount);

    e.as_contract(&vault_address, || {
        let shares = Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone());

        // Try to redeem more shares than user has
        Vault::redeem(&e, shares + 1, user.clone(), user.clone(), user.clone());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #401)")]
fn asset_address_already_set() {
    let e = Env::default();
    let asset_address1 = Address::generate(&e);
    let asset_address2 = Address::generate(&e);
    let vault_address = create_vault_contract(&e, &asset_address1, 6);

    e.as_contract(&vault_address, || {
        // Try to set asset address again (should panic)
        Vault::set_asset(&e, asset_address2);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #402)")]
fn decimals_offset_already_set() {
    let e = Env::default();
    let asset_address = Address::generate(&e);
    let vault_address = create_vault_contract(&e, &asset_address, 6);

    e.as_contract(&vault_address, || {
        // Try to set decimals offset again (should panic)
        Vault::set_decimals_offset(&e, 8);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #409)")]
fn decimals_offset_exceeded() {
    let e = Env::default();
    let asset_address = Address::generate(&e);

    // Try to set the offset to a value greater than MAX_DECIMALS_OFFSET
    let _ = create_vault_contract(&e, &asset_address, MAX_DECIMALS_OFFSET + 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn query_asset_not_set() {
    let e = Env::default();
    let contract_address = e.register(MockVaultContract, ());

    e.as_contract(&contract_address, || {
        // Try to query asset before setting it (should panic)
        Vault::query_asset(&e);
    });
}

#[test]
fn convert_zero_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        // Converting 0 assets should return 0 shares
        assert_eq!(Vault::convert_to_shares(&e, 0), 0);
        assert_eq!(Vault::preview_deposit(&e, 0), 0);
        assert_eq!(Vault::preview_withdraw(&e, 0), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #403)")]
fn invalid_assets_amount() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        // Try to convert negative assets (should panic)
        Vault::convert_to_shares(&e, -1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #404)")]
fn invalid_shares_amount() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);

    e.as_contract(&vault_address, || {
        // Try to convert negative shares (should panic)
        Vault::convert_to_assets(&e, -1);
    });
}

#[test]
fn deposit_transfer_from() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);

    e.mock_all_auths();

    // Admin approves operator to spend their tokens
    asset_client.approve(&admin, &operator, &deposit_amount, &1000);

    e.as_contract(&vault_address, || {
        // Operator deposits admin's assets to user (allowance-based transfer)
        let shares_minted =
            Vault::deposit(&e, deposit_amount, user.clone(), admin.clone(), operator.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_minted);
        assert_eq!(Base::total_supply(&e), shares_minted);
        assert_eq!(Vault::total_assets(&e), deposit_amount);

        // For first deposit, shares should equal assets with offset
        assert_eq!(shares_minted, deposit_amount * 10i128.pow(decimals_offset));
    });
}

#[test]
#[should_panic(expected = "Insufficient allowance")]
fn deposit_transfer_from_not_enough_allowance() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let deposit_amount = 100_000_000_000_000_000i128;
    let insufficient_allowance = 50_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);

    e.mock_all_auths();

    // Admin approves operator with insufficient allowance
    asset_client.approve(&admin, &operator, &insufficient_allowance, &1000);

    e.as_contract(&vault_address, || {
        // Try to deposit more than allowance (should panic)
        Vault::deposit(&e, deposit_amount, user.clone(), admin.clone(), operator.clone());
    });
}

#[test]
fn mint_transfer_from() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let shares_to_mint = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);

    e.mock_all_auths();

    // Calculate required assets outside vault context
    let required_assets = e.as_contract(&vault_address, || Vault::preview_mint(&e, shares_to_mint));

    // Admin approves operator to spend the required assets
    asset_client.approve(&admin, &operator, &required_assets, &1000);

    e.as_contract(&vault_address, || {
        // Operator mints shares for user using admin's assets (allowance-based
        // transfer)
        let assets_deposited =
            Vault::mint(&e, shares_to_mint, user.clone(), admin.clone(), operator.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_to_mint);
        assert_eq!(Base::total_supply(&e), shares_to_mint);
        assert_eq!(assets_deposited, required_assets);
    });
}

#[test]
#[should_panic(expected = "Insufficient allowance")]
fn mint_transfer_from_not_enough_allowance() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);
    let initial_supply = 1_000_000_000_000_000_000i128;
    let decimals_offset = 6;
    let shares_to_mint = 100_000_000_000_000_000i128;

    // Create contracts
    let asset_address = create_asset_contract(&e, initial_supply, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, decimals_offset);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);

    e.mock_all_auths();

    // Calculate required assets outside vault context
    let required_assets = e.as_contract(&vault_address, || Vault::preview_mint(&e, shares_to_mint));

    let insufficient_allowance = required_assets / 2;

    // Admin approves operator with insufficient allowance
    asset_client.approve(&admin, &operator, &insufficient_allowance, &1000);

    e.as_contract(&vault_address, || {
        // Try to mint with insufficient allowance (should panic)
        Vault::mint(&e, shares_to_mint, user.clone(), admin.clone(), operator.clone());
    });
}
