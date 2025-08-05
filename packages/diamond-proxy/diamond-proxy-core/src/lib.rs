#![no_std]

use soroban_sdk::contracterror;
pub use stellar_facet_macro::facet;

pub mod facets;
pub mod storage;
pub mod utils;

#[contracterror]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u8)]
pub enum Error {
    NotFound = 0,
    AlreadyExists = 1,
    InvalidInput = 2,
    InvalidState = 3,
    InvalidAddress = 4,
    InvalidAmount = 5,
    InsufficientBalance = 6,
    InsufficientAllowance = 7,
    InvalidSignature = 8,
    InvalidTimestamp = 9,
    InvalidDuration = 10,
    InvalidRate = 11,
    YieldNotFound = 12,
    YieldAlreadyClaimed = 13,
    ProposalNotFound = 14,
    TradeNotFound = 15,
    InvalidTradeStatus = 16,
    AlreadyInitialized = 17,
    // This occurs when the DiamondFactory is not used to properly setup the DiamondProxy.
    DiamondProxyNotInitialized = 18,
    FunctionNotFound = 19,
    Unauthorized = 20,
    InvalidArgument = 21,
    InitializationFailed = 22,
    FacetNotFound = 23,
    FacetAlreadyExists = 24,
    InvalidToken = 25,
    InvalidOperation = 26,
    ProposalNotActive = 27,
    InvalidProposal = 28,
    InvalidVote = 29,
    Uninitialized = 30,
    TradeNotActive = 31,
    DiamondExecFailed = 32,
    InvalidXdrSerialization = 33,
    ProviderNotFound = 34,
    VerificationNotFound = 35,
    VerificationExpired = 36,
    InvalidRequirement = 37,
    ProviderAlreadyExists = 38,
    ClaimNotFound = 39,
    SharedStorageNotInitialized = 40,
    DiamondInitFailed = 41,
    // Use replace if the intended functionality is to overwrite old facets
    DiamondSelectorAlreadyAdded = 42,
    DiamondSelectorNotFound = 43,
    // Security errors for direct call protection
    DiamondProxyNotSet = 44,
    UnauthorizedDirectCall = 45,
    OwnerNotSet = 46,
}
