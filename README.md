# OpenZeppelin Stellar Soroban Contracts

> [!Warning]
> This is experimental software and is provided on an "as is" and "as available" basis. We do not give any warranties and will not be liable for any losses incurred through any use of this code base.


OpenZeppelin Stellar Soroban Contracts is a collection of contracts for the Stellar network. Our goal is to bring Web3 standards under the OpenZeppelin quality by providing a set of high-quality, battle-tested contracts that can be used to build decentralized applications on the Stellar network.


## Project Structure

- `contracts/`: Source code
  - `token/`: Various token types (fungible, non-fungible, etc.)
  - `utils/`: Utilities for token types (pausable, etc.)
- `examples/`: Example contracts
- `docs/`: Documentation
- `audits/`: Audit reports

## Docs
We have a [documentation website](https://docs.openzeppelin.com/stellar-contracts/) explaining the high-level concepts of the library. You can find code-specific inline documentation in the source code, or alternatively you can locally generate the documentation using the `cargo doc --no-deps --lib --open` command, which will generate the documentation and open it using your default browser.


## Setup

Stellar smart contracts are programs written in Rust leveraging the [Soroban SDK](https://crates.io/crates/soroban-sdk). Please, follow the setup process as outlined in the [Stellar documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup).


## Usage

The library has not been published yet to `crates.io`, and this will be the case until we reach a stable version. However, one can [specify a git dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories) in a `Cargo.toml`, like so:

```toml
[dependencies]
openzeppelin-pausable = { git = "https://github.com/OpenZeppelin/stellar-contracts" }
openzeppelin-fungible-token = { git = "https://github.com/OpenZeppelin/stellar-contracts" }
```

We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.


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
