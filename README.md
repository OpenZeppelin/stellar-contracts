[![Coverage Status](https://codecov.io/gh/OpenZeppelin/openzeppelin-contracts/graph/badge.svg)](https://codecov.io/gh/OpenZeppelin/stellar-contracts)
[![Docs](https://img.shields.io/badge/docs-%F0%9F%93%84-yellow)](https://docs.openzeppelin.com/stellar-contracts)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/OpenZeppelin/stellar-contracts)

# OpenZeppelin Stellar Soroban Contracts

> [!Warning]
> This is experimental software and is provided on an "as is" and "as available" basis. We do not give any warranties and will not be liable for any losses incurred through any use of this code base.


OpenZeppelin Stellar Soroban Contracts is a collection of contracts for the Stellar network. Our goal is to bring Web3 standards under the OpenZeppelin quality by providing a set of high-quality, battle-tested contracts that can be used to build decentralized applications on the Stellar network.


## Project Structure

- `packages/`: Source code
  - [`access/`](packages/access): Role-based access controls and ownable
  - [`accounts/`](packages/accounts): Smart accounts with custom authentication and authorization
  - [`contract-utils/`](packages/contract-utils): Utilities for contracts (pausable, upgradeable, cryptography, etc.)
  - [`macros/`](packages/macros): Proc and derive macros for some of the modules (`#[only_owner]`, `#[when_not_paused]`, `#[derive(Upgradeable)]`, etc.)
  - [`test-utils/`](packages/test-utils): Utilities for testing
  - [`tokens/`](packages/tokens): Various token types (fungible, non-fungible, real-world assets, vaults)
- `examples/`: Example contracts
- `audits/`: Audit reports


## Docs
We have a [documentation website](https://docs.openzeppelin.com/stellar-contracts/) explaining the high-level concepts of the library. You can find code-specific inline documentation in the source code, or alternatively you can locally generate the documentation using the `cargo doc --no-deps --lib --open` command, which will generate the documentation and open it using your default browser.


## Setup

Stellar smart contracts are programs written in Rust leveraging the [Soroban SDK](https://crates.io/crates/soroban-sdk). Please, follow the setup process as outlined in the [Stellar documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup).


## How To Test/Play With Example Contracts
The below section is based on [Official Stellar Docs](https://developers.stellar.org/docs/build/smart-contracts/getting-started/hello-world). If you are stuck on any of the steps below, or want to dive in deeper, please refer to the official documentation.

We provide a set of example contracts that demonstrate how to use the library. You can find them in the `examples/` directory. If you want to deploy the example contracts to the testnet and play with them, you can follow the instructions below:
1. `git clone https://github.com/OpenZeppelin/stellar-contracts.git`
2. `cd stellar-contracts/examples`
3. Take a look at the current folder, and select an example contract you are interested in. We will go with the `fungible-pausable` in this guide.
4. `cd fungible-pausable`
5. `cargo build --target wasm32v1-none --release`
6. Now, the `target/wasm32v1-none/release/` directory will contain the compiled contracts. In this case, `target/wasm32v1-none/release/fungible_pausable_example.wasm` is the compiled wasm file.
7. Deploying to the testnet is no different than any other contract. You can follow the instructions in the [Stellar documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-to-testnet).


## How To Use This Library As A Dependency

Add the crate to your Cargo.toml:

We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.

```toml
[dependencies]
stellar-tokens = "=0.5.1"
stellar-access = "=0.5.1"
stellar-contract-utils = "=0.5.1"
```

## Notes

We strive to maintain consistency with other OpenZeppelin libraries as much as possible, following the same conventions and patterns. However, this Soroban library does not have a 1-to-1 mapping with the other OpenZeppelin contracts. Due to differences in the Stellar/Soroban architecture and capabilities, certain features and implementations may differ from their counterparts from other ecosystems.

## Security

For security concerns, please refer to our [Security Policy](SECURITY.md).


## License

OpenZeppelin Stellar Soroban Contracts are released under the [MIT LICENSE](LICENSE).


## Coding Standards

We try to follow the idiomatic Rust style, and enforce `clippy` and `cargo fmt` checks in CI.
The detailed rules are defined in the [.rustfmt.toml](./rustfmt.toml) and [.clippy.toml](./clippy.toml) files.


## Contributing

We welcome contributions from the community!

If you are looking for a good place to start, find a good first issue [here](https://github.com/OpenZeppelin/stellar-contracts/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22).

You can open an issue for a [bug report](https://github.com/OpenZeppelin/stellar-contracts/issues/new?template=bug_report.yml), [core implementation](https://github.com/OpenZeppelin/stellar-contracts/issues/new?template=core_implementation.yml), or [feature request](https://github.com/OpenZeppelin/stellar-contracts/issues/new?template=feature_request.ymll).

You can find more details in our [Contributing](CONTRIBUTING.md) guide, and please read our [Code of Conduct](CODE_OF_CONDUCT.md).
