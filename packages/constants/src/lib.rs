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
