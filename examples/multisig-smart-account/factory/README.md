# Smart Account Factory Example

Use this factory if you want to deploy accounts to deterministic addresses
derived from their initial signers and policies, or predict an address before
deployment.

This guide deploys the multisig smart account from the
[account example](../account) through a factory. Like the verifier and policy
examples, the factory is deployed once per network and referenced by address.

For more information about smart accounts and their components, check:
- [OpenZeppelin Stellar Contracts Documentation](https://docs.openzeppelin.com/stellar-contracts/accounts/smart-account)
- [Smart Accounts Package README](../../../packages/accounts/README.md)
- [Multisig Smart Account Example](../README.md) for deploying an account directly, without a factory

## Why a factory?

Soroban derives a contract address from `(network, deployer, chain_salt)`. The
factory is the deployer (`deployer().with_current_contract(chain_salt)`), so
only it can create in its namespace. Because the deployer is the factory rather
than the transaction caller, the account address is independent of who submits
or pays for the deployment transaction. The 32-byte `chain_salt` is computed as
`sha256` of the canonical XDR of `(signers, policies, salt)`, where `signers`
and `policies` are the account constructor arguments and `salt` is a
caller-chosen `u32`.

The account wasm hash is pinned on the factory instance during
`__constructor`. Because code hash is not part of a Stellar contract address,
pinning the wasm at factory construction prevents arbitrary bytecode from
landing at a predicted address.

## 1. Setup

Follow the [account example](../README.md) prerequisites to configure the
network, fund a `feepayer` identity, deploy the verifier and policy contracts,
and generate signer keys. The addresses and keys below are the sample ones from
that guide.

## 2. Upload the Account Wasm

The factory instantiates account code that is already uploaded on the network:

```bash
stellar contract upload \
    --wasm target/wasm32v1-none/release/multisig_account_example.wasm
```

The command prints the 32-byte wasm hash. Pass that value as
`--account_wasm_hash` when deploying the factory.

## 3. Deploy the Factory

Deploy the factory, pinning the account wasm hash from the previous step:

```bash
stellar contract deploy --alias account_factory \
    --wasm target/wasm32v1-none/release/multisig_account_factory_example.wasm \
    -- \
    --account_wasm_hash <wasm hash from the previous step>
```

Verify the pin:

```bash
stellar contract invoke --id account_factory -- pinned_account_wasm_hash
```

## 4. Predict the Account Address

Compute the address for a 2-of-2 account with the two Ed25519 keys from the
account example and the threshold policy. In the command below,
`CDLDYJWEZSM6IAI4HHPEZTTV65WX4OVN3RZD3U6LQKYAVIZTEK7XYAYT` is the Ed25519
verifier contract address, the two 32-byte hex strings are the Ed25519 public
keys, and `CA7IJLIHDBTE5S5EIMTIWRKKTSJP6KPH2VOU255CB2RNTWXQGYJRKKC3` is the
threshold policy contract address (replace these with your own deployed
addresses and keys when testing). The `salt` argument is `0` for the
first account with this configuration:

```bash
stellar contract invoke --id account_factory -- predict_address \
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

## 5. Deploy the Account

Deploy with the same tuple. The returned address is the one `predict_address` gave:

```bash
stellar contract invoke --id account_factory -- deploy \
    --signers '[ ...same as above... ]' \
    --policies '{ ...same as above... }' \
    --salt 0
```

Anyone may call `deploy`. The transaction requires only the fee payer's signature.

To create another account using the same initial signers and policies, use a
different `salt`.
