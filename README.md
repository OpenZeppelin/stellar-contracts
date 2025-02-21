# OpenZeppelin Stellar Soroban Contracts

> [!Warning]
> This project is still in a very early and experimental phase. It has never been audited nor thoroughly reviewed for security vulnerabilities. Use in production environments at your own risk.


OpenZeppelin Stellar Soroban Contracts is a collection of contracts for the Stellar network. Our goal is to bring Web3 standards under the OpenZeppelin quality by providing a set of high-quality, battle-tested contracts that can be used to build decentralized applications on the Stellar network.


## Project Structure

- `contracts/`: Source code
  - `token/`: Various token types (fungible, non-fungible, etc.)
  - `utils/`: Utilities for token types (pausable, etc.)
- `examples/`: Example contracts
- `docs/`: Documentation

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
