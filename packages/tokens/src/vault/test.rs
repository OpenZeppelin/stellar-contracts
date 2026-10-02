extern crate std;

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, MuxedAddress, String,
};

use crate::{
    fungible::{
        capped::{Capped, FungibleCapped},
        total_supply::{total_supply, FungibleTotalSupply, TotalSupply},
        Base, Compose, ContractOverrides, FungibleToken,
    },
    vault::{CappedVault, FungibleVault, Vault, VaultOverrides, MAX_DECIMALS_OFFSET},
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
        assert_eq!(total_supply(&e), shares_minted);
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
        assert_eq!(total_supply(&e), shares_to_mint);
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

    let shares_minted = e.as_contract(&vault_address, || {
        Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone())
    });

    e.as_contract(&vault_address, || {
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

    let shares_minted = e.as_contract(&vault_address, || {
        Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone())
    });

    e.as_contract(&vault_address, || {
        // Redeem half the shares
        let shares_to_redeem = shares_minted / 2;
        let assets_received =
            Vault::redeem(&e, shares_to_redeem, user.clone(), user.clone(), user.clone());

        // Check results
        assert_eq!(Base::balance(&e, &user), shares_minted - shares_to_redeem);
        assert_eq!(total_supply(&e), shares_minted - shares_to_redeem);

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
    });

    e.as_contract(&vault_address, || {
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

    let shares = e.as_contract(&vault_address, || {
        Vault::deposit(&e, deposit_amount, user.clone(), user.clone(), user.clone())
    });

    e.as_contract(&vault_address, || {
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
fn deposit_zero_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        let shares = Vault::deposit(&e, 0, admin.clone(), admin.clone(), admin.clone());
        assert_eq!(shares, 0);
        assert_eq!(total_supply(&e), 0);
        assert_eq!(Vault::total_assets(&e), 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #411)")]
fn deposit_zero_shares_after_donation() {
    let e = Env::default();
    let attacker = Address::generate(&e);
    let victim = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &attacker);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    asset_client.mint(&victim, &1_000);
    let vault_address = create_vault_contract(&e, &asset_address, 0);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Vault::deposit(&e, 1, attacker.clone(), attacker.clone(), attacker.clone());
    });
    asset_client.transfer(&attacker, &vault_address, &100);

    e.as_contract(&vault_address, || {
        assert_eq!(total_supply(&e), 1);
        assert_eq!(Vault::total_assets(&e), 101);
        assert_eq!(Vault::preview_deposit(&e, 50), 0);

        Vault::deposit(&e, 50, victim.clone(), victim.clone(), victim.clone());
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
        assert_eq!(total_supply(&e), shares_minted);
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
        assert_eq!(total_supply(&e), shares_to_mint);
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

// With offset `MAX_DECIMALS_OFFSET`, every conversion computes
// `total_supply + 10^offset`, so the vault reaches the `i128` ceiling once it
// holds `i128::MAX / 10^offset` asset base units.
fn virtual_shares() -> i128 {
    10_i128.pow(MAX_DECIMALS_OFFSET)
}

fn assets_at_virtual_bound() -> i128 {
    i128::MAX / virtual_shares()
}

#[test]
fn deposit_one_below_virtual_bound_keeps_vault_operational() {
    let e = Env::default();
    let user = Address::generate(&e);
    let boundary = assets_at_virtual_bound();
    let asset_address = create_asset_contract(&e, boundary, &user);
    let vault_address = create_vault_contract(&e, &asset_address, MAX_DECIMALS_OFFSET);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        let shares = Vault::deposit(&e, boundary - 1, user.clone(), user.clone(), user.clone());
        assert_eq!(shares, (boundary - 1) * virtual_shares());

        // `total_supply + 10^offset` is exactly representable here, so every
        // conversion still works.
        assert_eq!(Vault::preview_redeem(&e, virtual_shares()), 1);
        assert_eq!(Vault::preview_deposit(&e, 1), virtual_shares());
    });

    // Separate frame: `deposit` and `redeem` both `require_auth` for `user`.
    e.as_contract(&vault_address, || {
        assert_eq!(
            Vault::redeem(&e, virtual_shares(), user.clone(), user.clone(), user.clone()),
            1
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #410)")]
fn deposit_exceeding_virtual_bound_is_rejected() {
    let e = Env::default();
    let user = Address::generate(&e);
    let attacker = Address::generate(&e);
    let boundary = assets_at_virtual_bound();
    let asset_address = create_asset_contract(&e, boundary - 1, &user);
    MockAssetContractClient::new(&e, &asset_address).mint(&attacker, &1);
    let vault_address = create_vault_contract(&e, &asset_address, MAX_DECIMALS_OFFSET);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Vault::deposit(&e, boundary - 1, user.clone(), user.clone(), user.clone());

        // The mint alone would fit (`total_supply` would land exactly on
        // `boundary * 10^offset`), but `total_supply + 10^offset` would not,
        // and every later conversion would fail. The deposit has to be
        // rejected before the shares are minted.
        Vault::deposit(&e, 1, attacker.clone(), attacker.clone(), attacker.clone());
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #410)")]
fn mint_exceeding_virtual_bound_is_rejected() {
    let e = Env::default();
    let user = Address::generate(&e);
    let attacker = Address::generate(&e);
    let boundary = assets_at_virtual_bound();
    let asset_address = create_asset_contract(&e, boundary - 1, &user);
    MockAssetContractClient::new(&e, &asset_address).mint(&attacker, &1);
    let vault_address = create_vault_contract(&e, &asset_address, MAX_DECIMALS_OFFSET);

    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Vault::deposit(&e, boundary - 1, user.clone(), user.clone(), user.clone());

        // Same boundary through the `mint` entry point: `10^offset` shares
        // cost exactly one asset here and would land on the same supply.
        Vault::mint(&e, virtual_shares(), attacker.clone(), attacker.clone(), attacker.clone());
    });
}

// ################## CAPPED VAULT ##################

// The finding that motivated `CappedVault`: a vault listing `Capped` must not
// let a deposit mint shares past the cap.
#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_vault_deposit_rejects_exceeding_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            150,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn capped_vault_mint_rejects_exceeding_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        <CappedVault as VaultOverrides>::mint(&e, 60, admin.clone(), admin.clone(), admin.clone());
        <CappedVault as VaultOverrides>::mint(&e, 41, admin.clone(), admin.clone(), admin.clone());
    });
}

#[test]
fn capped_vault_reports_the_cap_in_its_limits() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        // empty vault, offset 0: one share per asset
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), 100);
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), 100);

        // depositing exactly the reported maximum succeeds and fills the cap
        let shares = <CappedVault as VaultOverrides>::deposit(
            &e,
            100,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
        assert_eq!(shares, 100);
        assert_eq!(total_supply(&e), 100);
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), 0);
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), 0);
    });
}

// With a decimals offset, the cap counts shares, not assets.
#[test]
fn capped_vault_limits_follow_the_decimals_offset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 6);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100 * 10i128.pow(6));
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), 100);

        <CappedVault as VaultOverrides>::deposit(
            &e,
            100,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
        assert_eq!(total_supply(&e), 100 * 10i128.pow(6));
    });
}

// At a share price that is not a whole number, the reported `max_deposit` is
// rounded down, so depositing it never mints past the cap.
#[test]
fn capped_vault_max_deposit_never_overshoots_the_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 10_000, &admin);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 2_000);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            1_000,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
    });
    // donation: 1_333 assets back 1_000 shares
    asset_client.mint(&vault_address, &333);

    e.as_contract(&vault_address, || {
        let max_assets = <CappedVault as VaultOverrides>::max_deposit(&e, admin.clone());
        // floor(1_000 * 1_334 / 1_001)
        assert_eq!(max_assets, 1_332);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            max_assets,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
        assert!(total_supply(&e) <= 2_000);
    });
}

// The cap may be lowered below the current supply (refer to `set_cap`); the
// limits then report zero instead of a negative amount.
#[test]
fn capped_vault_limits_are_zero_above_the_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            80,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
        Capped::set_cap(&e, 50);
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), 0);
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), 0);
    });
}

// A cap far above anything the vault can reach, with a share price high
// enough that converting the remaining shares overflows an `i128`: the view
// must not panic, and reports that the cap does not bind.
#[test]
fn capped_vault_max_deposit_does_not_overflow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 0, &admin);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    asset_client.mint(&vault_address, &10i128.pow(30));

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, i128::MAX);
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), i128::MAX);
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), i128::MAX);
    });
}

// When the vault's asset balance is `i128::MAX`, `deposit` overflows on
// `total_assets + 1` and cannot succeed, so the reported limit must be `0`
// rather than `i128::MAX`.
#[test]
fn capped_vault_max_deposit_is_zero_when_deposits_cannot_succeed() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 0, &admin);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    asset_client.mint(&vault_address, &i128::MAX);

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        // room remains under the cap, but no deposit can go through
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), 100);
        assert_eq!(<CappedVault as VaultOverrides>::max_deposit(&e, admin.clone()), 0);
    });
}

// Pins the claim above: a deposit in that state panics.
#[test]
#[should_panic(expected = "Error(Contract, #410)")]
fn capped_vault_deposit_panics_when_total_assets_overflow() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1, &admin);
    let asset_client = MockAssetContractClient::new(&e, &asset_address);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    asset_client.mint(&vault_address, &i128::MAX);

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            1,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
    });
}

// The share token's decimals stay the vault's (underlying decimals plus the
// offset) on the capped contract type.
#[test]
fn capped_vault_decimals_are_the_vault_decimals() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 6);

    e.as_contract(&vault_address, || {
        assert_eq!(<CappedVault as ContractOverrides>::decimals(&e), Vault::decimals(&e));
        assert_eq!(<CappedVault as ContractOverrides>::decimals(&e), 18 + 6);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]
fn capped_vault_deposit_requires_a_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        <CappedVault as VaultOverrides>::deposit(
            &e,
            10,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
    });
}

// Withdrawals and redemptions are not touched by the cap and keep the vault
// defaults, freeing room under the cap again.
#[test]
fn capped_vault_redeem_frees_room_under_the_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = create_vault_contract(&e, &asset_address, 0);
    e.mock_all_auths();

    e.as_contract(&vault_address, || {
        Capped::set_cap(&e, 100);
        <CappedVault as VaultOverrides>::deposit(
            &e,
            100,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
    });
    e.as_contract(&vault_address, || {
        <CappedVault as VaultOverrides>::redeem(
            &e,
            40,
            admin.clone(),
            admin.clone(),
            admin.clone(),
        );
        assert_eq!(total_supply(&e), 60);
        assert_eq!(<CappedVault as VaultOverrides>::max_mint(&e, admin.clone()), 40);
        assert_eq!(
            <CappedVault as VaultOverrides>::max_withdraw(&e, admin.clone()),
            Vault::max_withdraw(&e, admin.clone())
        );
    });
}

// A contract composing the capped vault: `FungibleVault` and `FungibleCapped`
// both accept `CappedVault`, and the `FungibleVault` default bodies reach the
// capped entry points through `VaultOverrides`.
#[contract]
struct CappedVaultContract;

#[contractimpl]
impl CappedVaultContract {
    pub fn __constructor(e: &Env, asset: Address, cap: i128) {
        Vault::set_asset(e, asset);
        Vault::set_decimals_offset(e, 0);
        Capped::set_cap(e, cap);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for CappedVaultContract {
    type ContractType = Compose<(Vault, Capped, TotalSupply)>;

    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

#[contractimpl(contracttrait)]
impl FungibleTotalSupply for CappedVaultContract {}

#[contractimpl(contracttrait)]
impl FungibleVault for CappedVaultContract {}

#[contractimpl(contracttrait)]
impl FungibleCapped for CappedVaultContract {}

#[test]
fn capped_vault_contract_enforces_the_cap_through_the_trait() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let asset_address = create_asset_contract(&e, 1_000, &admin);
    let vault_address = e.register(CappedVaultContract, (asset_address, 100_i128));
    let client = CappedVaultContractClient::new(&e, &vault_address);

    assert_eq!(client.cap(), 100);
    assert_eq!(client.max_deposit(&admin), 100);
    assert_eq!(client.max_mint(&admin), 100);

    client.deposit(&60, &admin, &admin, &admin);
    assert_eq!(client.total_supply(), 60);
    assert_eq!(client.max_mint(&admin), 40);

    // past the cap, through both share-creating entry points
    assert!(client.try_deposit(&41, &admin, &admin, &admin).is_err());
    assert!(client.try_mint(&41, &admin, &admin, &admin).is_err());

    client.mint(&40, &admin, &admin, &admin);
    assert_eq!(client.total_supply(), 100);

    // transfers of shares are unaffected by the cap
    let bob = Address::generate(&e);
    client.transfer(&admin, MuxedAddress::from(bob.clone()), &10);
    assert_eq!(client.balance(&bob), 10);
}
