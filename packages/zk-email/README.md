# Stellar zkEmail

> [!Warning]
> This package is under development and not ready for use. Its API may change without notice, and it is not published on crates.io.

zkEmail utilities for Stellar contracts: on-chain email verification using zero-knowledge proofs of DKIM signatures.

## Modules

### DKIM Registry

The `dkim_registry` module is the on-chain trust anchor for a zkEmail system. It stores which DKIM public key hashes are valid for which domains, so that a proof verifier can confirm the key that signed an email is still trusted.

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
