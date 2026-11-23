# Stellar Finance

Finance utilities for Stellar contracts

This crate is part of the [OpenZeppelin Stellar Contracts](https://github.com/OpenZeppelin/stellar-contracts) library, which is published as separate crates on [crates.io](https://crates.io):

- [stellar-access](https://crates.io/crates/stellar-access): Role-based access controls and ownable
- [stellar-accounts](https://crates.io/crates/stellar-accounts): Smart accounts with custom authentication and authorization
- [stellar-confidential](https://crates.io/crates/stellar-confidential): Confidential token with encrypted balances and zero-knowledge proofs
- [stellar-contract-utils](https://crates.io/crates/stellar-contract-utils): Utilities for contracts (pausable, upgradeable, cryptography, etc.)
- [stellar-fee-abstraction](https://crates.io/crates/stellar-fee-abstraction): Fee abstraction utilities
- **[stellar-finance](https://crates.io/crates/stellar-finance)**: Finance utilities (vesting)
- [stellar-governance](https://crates.io/crates/stellar-governance): Governance utilities (governor, votes, timelock)
- [stellar-macros](https://crates.io/crates/stellar-macros): Proc macros (`#[only_owner]`, `#[when_not_paused]`, etc.)
- [stellar-tokens](https://crates.io/crates/stellar-tokens): Token types (fungible, non-fungible, real-world assets, vaults)

## Overview

This package provides finance modules for Soroban smart contracts:

- **Vesting**: a vesting contract that holds SEP-41 tokens and releases them to a single receiver on a fixed schedule

## Modules

### Vesting

The `vesting` module implements a trustless vesting contract. Any SEP-41 token sent to it (including XLM through its Stellar Asset Contract) is released to the receiver following one schedule: nothing before the cliff, then a linear curve, then everything from the end of the schedule.

- **Receiver**: the `Ownable` owner of the vesting contract. Ownership can be transferred in two steps; the examples disable renouncing.
- **Release**: `release(token)` is permissionless and always pays the current owner.
- **Custom schedules**: overriding the single `vesting_curve` method; the library still applies the cliff, the end, and the bounds checks.

#### How a vesting contract works

1. **Deploy.** The constructor sets the receiver and the schedule. The schedule is made of absolute dates (`start`, `cliff`, `end`), fixed at deployment and never changed afterwards.
2. **Fund.** Tokens are sent to the vesting contract's address with a plain transfer on the token contract, exactly like sending to any other address. For example, funding with XLM means sending XLM to the vesting contract's address. Anyone can fund, any number of times, and no function of the vesting contract is called.
3. **Release.** As time passes, tokens vest according to the schedule. Anyone can call `release(token)` at any time, and the amount vested so far (minus what was already released) is sent to the receiver.

**Funding does not start the schedule.** The schedule runs on its own dates, set at deployment, whether the vesting contract holds any tokens or not. A transfer to the vesting contract only runs the token contract's code: the vesting contract is not called, there is no hook and no notification, so it cannot know when, or whether, it received tokens. It only sees its current balance, read from the token contract whenever it computes an amount. A late (or forgotten and later sent) funding therefore does not shift the schedule: tokens sent after `start` are partly vested as soon as they arrive, fully vested if sent after `end`, and only tokens sent before `start` follow the full schedule.

**Several tokens, one schedule.** Any SEP-41 token can be sent to the same vesting contract, and each one is released separately with `release(token)`. The amount already released is tracked per token, but every token follows the same single schedule.

**Several schedules: one vesting contract each.** A vesting contract holds exactly one schedule. To vest with a different schedule (other dates or another curve), another vesting contract is deployed with that schedule. To have a second, independent instance of the same schedule (for another receiver, or a grant tracked apart), another copy is deployed as well. Copies share the same uploaded Wasm, so each deployment is cheap.

```rust
use soroban_sdk::{contract, contracterror, contractimpl, panic_with_error, Address, Env};
use stellar_access::ownable::{set_owner, Ownable};
// `VestingSchedule` must be in scope: the default `vesting_curve` body is
// expanded in this module.
use stellar_finance::vesting::{set_schedule, VestingSchedule, VestingWallet};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleError {
    RenounceDisabled = 1,
}

#[contract]
pub struct VestingWalletContract;

#[contractimpl]
impl VestingWalletContract {
    pub fn __constructor(e: &Env, receiver: Address, start: u64, duration: u64, cliff_duration: u64) {
        set_owner(e, &receiver);
        set_schedule(e, start, duration, cliff_duration);
    }
}

#[contractimpl(contracttrait)]
impl VestingWallet for VestingWalletContract {}

#[contractimpl(contracttrait)]
impl Ownable for VestingWalletContract {
    // Without an owner, `release` has nowhere to send tokens, so renouncing
    // would lock every unreleased token forever.
    fn renounce_ownership(e: &Env) {
        panic_with_error!(e, ExampleError::RenounceDisabled);
    }
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-finance = "=0.8.0"
stellar-access = "=0.8.0"
```

## Examples

See the following examples in the repository:
- [`examples/vesting-wallet/linear/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/vesting-wallet/linear) - Linear schedule, renounce disabled
- [`examples/vesting-wallet/monthly-steps/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/vesting-wallet/monthly-steps) - Custom `vesting_curve` unlocking in monthly pieces

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
