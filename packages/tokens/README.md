# Stellar Tokens

Fungible and NonFungible Tokens for the Stellar contracts.

## Modules

### Fungible

The `fungible` module provides functionalities for fungible tokens: balance management, transfer operations, allowance delegation, total supply tracking.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_tokens::fungible::{burnable::FungibleBurnable, Base, ContractOverrides, FungibleToken};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_macros::{default_impl, only_owner};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn __constructor(e: &Env, initial_owner: Address) {
        // Set token metadata
        Base::set_metadata(
            e,
            8, // 8 decimals
            String::from_str(e, "My Token"),
            String::from_str(e, "MTK"),
        );

        // Set the contract owner
        ownable::set_owner(e, &initial_owner);
    }

    #[only_owner]
    pub fn mint_tokens(e: &Env, to: Address, amount: i128) {
        // Mint tokens to the recipient
        Base::mint(e, &to, amount);
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for MyContract {
    type ContractType = Base;
}

#[default_impl]
#[contractimpl]
impl FungibleBurnable for MyContract {}
```

#### Extensions

- **Burnable**: Allow token holders to destroy their tokens
- **Capped**: Set maximum supply limits
- **Allowlist**: Restrict transfers to approved addresses
- **Blocklist**: Prevent transfers from/to blocked addresses
- **Vault**: Enable deposit/withdrawal of underlying assets in exchange for vault shares

### Non-Fungible

The `non_fungible` module implements non-fungible token functionality:

- **Unique Token IDs**: Each token has a unique identifier
- **Ownership Tracking**: Track which account owns each token
- **Approval System**: Approve others to transfer specific tokens
- **Metadata Support**: Store name, symbol, and token URI

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::non_fungible::{
    burnable::NonFungibleBurnable,
    Base, ContractOverrides, NonFungibleToken,
};

#[contract]
pub struct MyNFTContract;

#[contractimpl]
impl MyNFTContract {
    pub fn __constructor(e: &Env) {
        Base::set_metadata(
            e,
            String::from_str(e, "www.mygame.com"),
            String::from_str(e, "My Game Items Collection"),
            String::from_str(e, "MGMC"),
        );
    }

    pub fn award_item(e: &Env, to: Address) -> u32 {
        // access control might be needed
        Base::sequential_mint(e, &to)
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for MyNFTContract {
    type ContractType = Base;
}

- [`examples/fungible-merkle-airdrop/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-merkle-airdrop) - Airdrop with merkle proofs
#[default_impl]
#[contractimpl]
impl NonFungibleBurnable for MyNFTContract {}
```

#### Extensions

- **Burnable**: Allow token holders to destroy their NFTs
- **Enumerable**: Enable iteration over all tokens and owner tokens
- **Consecutive**: Efficiently mint multiple tokens in batches
- **Royalties**: Support for creator royalties on secondary sales

## Design Philosophy

Both modules follow a **dual-layered approach**:

1. **High-Level Functions**: Include all necessary checks, verifications, authorizations, and event emissions. Perfect for standard use cases.

2. **Low-Level Functions**: Provide granular control for custom workflows. Require manual handling of verifications and authorizations.

This design allows developers to choose between convenience and customization based on their project requirements.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-tokens = "=0.5.1"
# Add this if you want to use macros
stellar-macros = "=0.5.1"
```

## Examples

See the following examples in the repository:
- [`examples/fungible-pausable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-pausable) - Pausable fungible token
- [`examples/nft-sequential-minting/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/nft-sequential-minting) - Basic non-fungible token

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
