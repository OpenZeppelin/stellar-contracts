#![no_std]

use soroban_sdk as _; // Import soroban-sdk for its panic handler

// Same values as in Stellar Asset Contract (SAC) implementation:
// https://github.com/stellar/rs-soroban-env/blob/main/soroban-env-host/src/builtin_contracts/stellar_asset_contract/storage_types.rs
pub const DAY_IN_LEDGERS: u32 = 17280;

pub const INSTANCE_EXTEND_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
pub const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const BALANCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const BALANCE_TTL_THRESHOLD: u32 = BALANCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const OWNER_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const OWNER_TTL_THRESHOLD: u32 = OWNER_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const OWNERSHIP_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const OWNERSHIP_TTL_THRESHOLD: u32 = OWNERSHIP_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const TOKEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const TOKEN_TTL_THRESHOLD: u32 = TOKEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const ROLE_EXTEND_AMOUNT: u32 = 90 * DAY_IN_LEDGERS;
pub const ROLE_TTL_THRESHOLD: u32 = ROLE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

pub const ADMIN_TRANSFER_TTL: u32 = 2 * DAY_IN_LEDGERS;
pub const ADMIN_TRANSFER_THRESHOLD: u32 = DAY_IN_LEDGERS;

// Merkle Distributor TTL constants
pub const MERKLE_CLAIMED_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const MERKLE_CLAIMED_TTL_THRESHOLD: u32 = MERKLE_CLAIMED_EXTEND_AMOUNT - DAY_IN_LEDGERS;
