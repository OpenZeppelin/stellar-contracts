# Smart Account Factory Example

This guide demonstrates how to deploy the multisig smart account from the
[account example](../account) through a factory that binds each account's
address to its initial configuration. Like the verifier and policy examples,
the factory is deployed once per network and referenced by address.

For more information about smart accounts and their components, check:
- [OpenZeppelin Stellar Contracts Documentation](https://docs.openzeppelin.com/stellar-contracts/accounts/smart-account)
- [Smart Accounts Package README](../../../packages/accounts/README.md)
- [Multisig Smart Account Example](../README.md) for deploying an account directly, without a factory

## Why a factory?

Soroban derives a contract address from `(network, deployer, salt)` and
nothing else. The wasm hash and the constructor arguments are not part of it.
When a wallet shows a user the address their account *will* have before it
exists, two things can go wrong:

- **Address squatting.** If the deployer is a regular `G...` account, anyone
  who can reproduce the salt can create a different contract at the predicted
  address first.
- **Configuration swap.** If the salt is unrelated to the account's
  configuration, someone can create *an* account at the predicted address with
  *their* signers.

The factory closes both gaps:

- Accounts are created with `deployer().with_current_contract(salt)`, so the
  deployer in the address preimage is the factory itself. Only the factory can
  create contracts in its namespace, and the factory has no `__check_auth`, so
  there is no key that could authorize a deployment from outside of it.
- The 32-byte chain salt is `sha256` of the canonical XDR of
  `(version, signers, policies, salt)`, where `signers` and `policies` are
  exactly the arguments passed to the account's `__constructor`. A different
  configuration is a different address.

## Interface

```
__constructor(account_wasm_hash: BytesN<32>)

predict(signers: Vec<Signer>, policies: Map<Address, Val>, salt: u32) -> Address
deploy_account(signers: Vec<Signer>, policies: Map<Address, Val>, salt: u32) -> Address

pinned_account_wasm_hash() -> BytesN<32>
```

The factory is generic over the account's constructor: `signers` and
`policies` are passed through unchanged, so a 2-of-3 multisig is three signers
plus the threshold policy address mapped to `{threshold: 2}`, exactly as in the
[account example](../README.md#5-deploy-the-multisig-smart-account).

`predict` and `deploy_account` take the same argument tuple. The `salt: u32`
is a caller-chosen value that lets several accounts share one configuration.
It is not the chain salt; the caller never supplies the chain salt, and there
is no view that exposes it. `predict` is the only address authority a client
needs.

The account wasm hash is pinned when the factory is constructed. It is not an
argument of `predict` or `deploy_account`, and there is no admin or setter to
change it. A different account wasm means a different factory, which is a
different deployer and therefore a disjoint set of addresses.

## 1. Setup

Follow the [Setup](../README.md#1-setup) and [Verifier Contracts](../README.md#2-verifier-contracts)
steps of the account example so that you have built the WASM binaries,
configured testnet, funded a `feepayer` identity, and deployed the verifier and
policy contracts you intend to use. The addresses below are the ones from that
guide.

## 2. Upload the Account Wasm

The factory instantiates account code that is already uploaded on the network.
Upload the account wasm and keep its hash:

```bash
stellar contract upload \
    --wasm target/wasm32v1-none/release/multisig_account_example.wasm
```

The command prints the 32-byte wasm hash in hex. For this example, we assume
it is:

```
fcc7a7e3e55d37797856417e342edd7133769498b267a3e9594c8615e428149a
```

## 3. Deploy the Factory

Deploy the factory, pinning the account wasm hash:

```bash
stellar contract deploy --alias account_factory \
    --wasm target/wasm32v1-none/release/multisig_account_factory_example.wasm \
    -- \
    --account_wasm_hash fcc7a7e3e55d37797856417e342edd7133769498b267a3e9594c8615e428149a
```

Verify the pin:

```bash
stellar contract invoke --id account_factory -- pinned_account_wasm_hash
```

> **Note:** Nothing is validated at construction. Pinning a hash whose code is
> not uploaded on this network yields a factory whose every `deploy_account`
> call fails. Check the pin before publishing the factory address.

## 4. Predict the Account Address

Compute the address for a 2-of-2 account with the two Ed25519 keys from the
account example and the threshold policy. The `salt` argument is `0` for the
user's first account with this configuration:

```bash
stellar contract invoke --id account_factory -- predict \
    --signers '[
        {
            "External": [
                "CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT",
                "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"
            ]
        },
        {
            "External": [
                "CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT",
                "4cb5abf6ad79fbf5abbccafcc269d85cd2651ed4b885b5869f241aedf0a5ba29"
            ]
        }
    ]' \
    --policies '{"CA7IJLIHDBTE5S5EIMTIWRKKTSJP6KPH2VOU255CB2RNTWXQGYJRKKC3": {"map": [{"key": {"symbol": "threshold"}, "val": {"u32": 2}}]}}' \
    --salt 0
```

`predict` is a pure computation. It answers for any tuple, including ones the
account contract refuses (an empty signer list, more than `MAX_SIGNERS`
signers, key data over `MAX_EXTERNAL_KEY_SIZE` bytes). Such an address cannot
be squatted, but funds sent to it are unrecoverable, so validate the
configuration against the account contract's limits before presenting the
address as usable.

## 5. Deploy the Account

Deploy with the same tuple. The returned address is the one `predict` gave:

```bash
stellar contract invoke --id account_factory -- deploy_account \
    --signers '[ ...same as above... ]' \
    --policies '{ ...same as above... }' \
    --salt 0
```

Anyone may call `deploy_account`, and no authorization is required: paying the
fee for somebody else's account is a supported use. During construction the
account authorizes its own policy installs as their direct invoker, so the
transaction needs no authorization entries beyond the fee payer's signature.

To give the same signers a second account, change only `salt`.

## What the tests prove

`cargo test --package multisig-account-factory-example` runs against the
account wasm in `testdata/`, so the tests read the deployed configuration back
off a real account. They cover:

- `predict` and `deploy_account` agree, and the account holds exactly the
  requested signers and policies with the given install parameters.
- The chain salt is `sha256` of the canonical XDR of
  `(version, signers, policies, salt)`, over the signer order the account
  actually holds. Mirroring the preimage over a non-canonical order does not
  reproduce the address.
- `[A, B]` and `[B, A]` are one address; so are `[A, A]` and `[A]`. Policy map
  order is canonical by construction.
- Every part of the tuple moves the address: signer set, policy address,
  policy parameters, extra salt, and signer type.
- Deploying the same tuple twice fails with an untyped
  `Error(Context, InvalidAction)`, and the predicted address is unchanged. A
  front-runner who submits your tuple creates your account, at your address,
  with your signers.
- A contract that is not the factory cannot create in the factory's namespace,
  while the identical operation in its own namespace succeeds.
- Every deployment uses the pinned wasm. A factory pinned to a different hash
  is a different namespace, and there is no argument through which a caller
  can supply other code.

## Notes

- **Collisions fail, on purpose.** Soroban has no contract-callable existence
  check, and the host's refusal to create at an occupied address cannot be
  caught inside the factory, so the factory does not keep a storage marker to
  fake idempotency. The failure reaches the caller as an untyped
  `Error(Context, InvalidAction)`. Probe the predicted address off-chain before
  submitting.
- **Simulation does not enforce contract authorization.** A direct
  `CreateContract` operation naming the factory as deployer simulates cleanly
  and is rejected only when the ledger applies it. The Rust test reproduces the
  rejection because the test host enforces authorization; when checking the
  property on a live network, wait for the ledger result rather than trusting
  the simulation.
- **Host map ordering is load-bearing.** Signer canonicalization relies on the
  host's ordering of map keys. That ordering feeds XDR and therefore ledger
  state hashes, so changing it would be a protocol break.
- **Keep the factory alive.** The factory's instance entry and the uploaded
  account code are subject to state archival like any other ledger entry.
  Extend their TTL periodically with `stellar contract extend`; the factory
  does not do this itself.
- **The account contract sets the ceiling.** The account's `__constructor`
  creates one default rule named `multisig` without expiry. The factory cannot
  create richer initial configurations than the account constructor accepts.
